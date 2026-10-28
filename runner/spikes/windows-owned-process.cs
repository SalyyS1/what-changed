using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

namespace OwnveroSpike
{
    // Feasibility only. Owns a cooperative child tree; hostile-code isolation is unproven.
    // API reference: https://learn.microsoft.com/windows/win32/procthread/job-objects
    public sealed class OwnedProcess : IDisposable
    {
        private SafeKernelHandle job;
        private SafeKernelHandle process;
        public int ProcessId { get; private set; }
        public int ExitCode
        {
            get
            {
                EnsureOpen();
                if (!WaitForExit(0)) throw new InvalidOperationException("Process is still running.");
                uint value;
                Check(Native.GetExitCodeProcess(process, out value), "GetExitCodeProcess");
                return unchecked((int)value);
            }
        }
        private OwnedProcess(SafeKernelHandle job, SafeKernelHandle process, int pid)
        { this.job = job; this.process = process; ProcessId = pid; }

        public static OwnedProcess Start(string executable, string[] arguments, string cwd)
        { return Start(executable, arguments, cwd, new Dictionary<string, string>()); }

        public static OwnedProcess Start(string executable, string[] arguments, string cwd,
            IDictionary<string, string> environment)
        {
            executable = AbsoluteLocalPath(executable);
            cwd = AbsoluteLocalPath(cwd);
            if (!File.Exists(executable) || !Directory.Exists(cwd))
                throw new ArgumentException("Executable and working directory must exist.");
            if (arguments == null || environment == null) throw new ArgumentNullException();
            var command = new StringBuilder(Quote(executable));
            foreach (string argument in arguments) command.Append(' ').Append(Quote(argument));
            if (command.Length >= 32767) throw new ArgumentException("Command exceeds Windows limit.");
            string block = EnvironmentBlock(environment);
            var ownedJob = Native.CreateJobObjectW(IntPtr.Zero, null);
            if (ownedJob.IsInvalid) { ownedJob.Dispose(); throw Failure("CreateJobObjectW"); }
            var info = new Native.ExtendedLimits();
            info.BasicLimitInformation.LimitFlags = 0x00002000; // KILL_ON_JOB_CLOSE only; no breakaway.
            Native.ProcessInformation child = new Native.ProcessInformation();
            IntPtr environmentPointer = IntPtr.Zero;
            bool ownershipTransferred = false;
            try
            {
                Check(Native.SetHandleInformation(ownedJob, 1, 0), "SetHandleInformation");
                Check(Native.SetInformationJobObject(ownedJob, 9, ref info,
                    (uint)Marshal.SizeOf(typeof(Native.ExtendedLimits))), "SetInformationJobObject");
                environmentPointer = Marshal.StringToHGlobalUni(block);
                var startup = new Native.StartupInfo();
                startup.cb = Marshal.SizeOf(typeof(Native.StartupInfo));
                // Explicit image/cwd/environment, no handle inheritance, shell or breakaway.
                Check(Native.CreateProcessW(executable, command, IntPtr.Zero, IntPtr.Zero, false,
                    0x00000004 | 0x00000400 | 0x08000000, environmentPointer, cwd,
                    ref startup, out child), "CreateProcessW");
                Check(Native.AssignProcessToJobObject(ownedJob, child.Process), "AssignProcessToJobObject");
                if (Native.ResumeThread(child.Thread) == UInt32.MaxValue) throw Failure("ResumeThread");
                int pid = checked((int)child.ProcessId);
                var ownedChild = new SafeKernelHandle(child.Process);
                child.Process = IntPtr.Zero;
                var result = new OwnedProcess(ownedJob, ownedChild, pid);
                ownershipTransferred = true;
                return result;
            }
            catch
            {
                // A failed assignment must never allow the suspended image to run.
                if (child.Process != IntPtr.Zero)
                {
                    if (!Native.TerminateProcess(child.Process, 1))
                        throw Failure("TerminateProcess after failed launch");
                    Native.WaitForSingleObject(child.Process, 5000);
                }
                throw;
            }
            finally
            {
                if (environmentPointer != IntPtr.Zero) Marshal.FreeHGlobal(environmentPointer);
                if (child.Thread != IntPtr.Zero) Native.CloseHandle(child.Thread);
                if (child.Process != IntPtr.Zero) Native.CloseHandle(child.Process);
                // Ownership transfers only after the suspended child is assigned and resumed.
                if (!ownershipTransferred) ownedJob.Dispose();
            }
        }

        public bool WaitForExit(int milliseconds)
        {
            EnsureOpen();
            if (milliseconds < -1) throw new ArgumentOutOfRangeException("milliseconds");
            uint result = Native.WaitForSingleObject(process, unchecked((uint)milliseconds));
            if (result == UInt32.MaxValue) throw Failure("WaitForSingleObject");
            return result == 0;
        }

        public void Terminate() { EnsureOpen(); Check(Native.TerminateJobObject(job, 1), "TerminateJobObject"); }

        // Snapshot only: membership may change immediately after this query.
        public int[] ActiveProcessIds()
        {
            EnsureOpen();
            for (int capacity = 16; capacity <= 65536; capacity *= 2)
            {
                int size = checked(8 + capacity * IntPtr.Size);
                IntPtr buffer = Marshal.AllocHGlobal(size);
                try
                {
                    uint returned;
                    bool success = Native.QueryInformationJobObject(job, 3, buffer, (uint)size, out returned);
                    int error = Marshal.GetLastWin32Error();
                    if (!success && error != 234) throw new Win32Exception(error, "QueryInformationJobObject");
                    if (!success) continue;
                    int assigned = Marshal.ReadInt32(buffer, 0), count = Marshal.ReadInt32(buffer, 4);
                    if (assigned > count) continue;
                    if (count < 0 || count > capacity) throw new InvalidOperationException("Invalid job process count.");
                    var ids = new int[count];
                    for (int i = 0; i < count; i++) ids[i] = checked((int)Marshal.ReadIntPtr(buffer, 8 + i * IntPtr.Size).ToInt64());
                    return ids;
                }
                finally { Marshal.FreeHGlobal(buffer); }
            }
            throw new InvalidOperationException("Job process query exceeded spike limit.");
        }

        public static bool CurrentProcessIsInJob()
        { bool value; Check(Native.IsProcessInJob(Native.GetCurrentProcess(), IntPtr.Zero, out value), "IsProcessInJob"); return value; }

        public static bool CurrentProcessIsElevated()
        {
            IntPtr token;
            Check(Native.OpenProcessToken(Native.GetCurrentProcess(), 8, out token), "OpenProcessToken");
            try
            {
                int value, length;
                Check(Native.GetTokenInformation(token, 20, out value, 4, out length), "GetTokenInformation");
                return value != 0;
            }
            finally { Native.CloseHandle(token); }
        }

        public void Dispose()
        {
            if (job == null) return;
            job.Dispose(); // Last noninherited handle closes: Windows kills all job descendants.
            process.Dispose(); job = null; process = null;
        }
        private void EnsureOpen() { if (job == null) throw new ObjectDisposedException("OwnedProcess"); }
        private static void Check(bool success, string operation) { if (!success) throw Failure(operation); }
        private static Win32Exception Failure(string operation) { return new Win32Exception(Marshal.GetLastWin32Error(), operation); }
        private static string AbsoluteLocalPath(string path)
        {
            if (String.IsNullOrEmpty(path) || path.Length < 3 || path[1] != ':' ||
                (path[2] != '\\' && path[2] != '/') || path.IndexOf('\0') >= 0)
                throw new ArgumentException("An absolute local drive path is required.");
            return Path.GetFullPath(path);
        }
        private static string EnvironmentBlock(IDictionary<string, string> environment)
        {
            var sorted = new SortedDictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            foreach (var item in environment)
            {
                if (String.IsNullOrEmpty(item.Key) || item.Key.IndexOfAny(new[] { '=', '\0' }) >= 0 ||
                    item.Value == null || item.Value.IndexOf('\0') >= 0) throw new ArgumentException("Invalid environment entry.");
                sorted.Add(item.Key, item.Value);
            }
            var block = new StringBuilder();
            foreach (var item in sorted) block.Append(item.Key).Append('=').Append(item.Value).Append('\0');
            if (block.Length == 0) block.Append('\0');
            return block.Append('\0').ToString(); // Non-null, double-NUL block even for an empty environment.
        }
        private static string Quote(string value)
        {
            if (value == null || value.IndexOf('\0') >= 0) throw new ArgumentException("Invalid argument.");
            var quoted = new StringBuilder("\"");
            int slashes = 0;
            foreach (char character in value)
            {
                if (character == '\\') { slashes++; continue; }
                quoted.Append('\\', character == '"' ? slashes * 2 + 1 : slashes).Append(character);
                slashes = 0;
            }
            return quoted.Append('\\', slashes * 2).Append('"').ToString();
        }
        private sealed class SafeKernelHandle : SafeHandleZeroOrMinusOneIsInvalid
        {
            public SafeKernelHandle() : base(true) { }
            public SafeKernelHandle(IntPtr handle) : base(true) { SetHandle(handle); }
            protected override bool ReleaseHandle() { return Native.CloseHandle(handle); }
        }

        private static class Native
        {
            [StructLayout(LayoutKind.Sequential)] internal struct BasicLimits
            { internal long PerProcessUserTimeLimit, PerJobUserTimeLimit; internal uint LimitFlags; internal UIntPtr MinimumWorkingSetSize, MaximumWorkingSetSize; internal uint ActiveProcessLimit; internal UIntPtr Affinity; internal uint PriorityClass, SchedulingClass; }
            [StructLayout(LayoutKind.Sequential)] internal struct IoCounters
            { internal ulong ReadOperationCount, WriteOperationCount, OtherOperationCount, ReadTransferCount, WriteTransferCount, OtherTransferCount; }
            [StructLayout(LayoutKind.Sequential)] internal struct ExtendedLimits
            { internal BasicLimits BasicLimitInformation; internal IoCounters IoInfo; internal UIntPtr ProcessMemoryLimit, JobMemoryLimit, PeakProcessMemoryUsed, PeakJobMemoryUsed; }
            [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)] internal struct StartupInfo
            { internal int cb; internal string reserved, desktop, title; internal uint x, y, xSize, ySize, xCountChars, yCountChars, fillAttribute, flags; internal ushort showWindow, reservedSize; internal IntPtr reservedPointer, standardInput, standardOutput, standardError; }
            [StructLayout(LayoutKind.Sequential)] internal struct ProcessInformation
            { internal IntPtr Process, Thread; internal uint ProcessId, ThreadId; }
            [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] internal static extern SafeKernelHandle CreateJobObjectW(IntPtr attributes, string name);
            [DllImport("kernel32.dll", SetLastError = true)] internal static extern bool SetHandleInformation(SafeKernelHandle handle, uint mask, uint flags);
            [DllImport("kernel32.dll", SetLastError = true)] internal static extern bool SetInformationJobObject(SafeKernelHandle job, int kind, ref ExtendedLimits info, uint size);
            [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] internal static extern bool CreateProcessW(string executable, StringBuilder command, IntPtr processAttributes, IntPtr threadAttributes, bool inheritHandles, uint flags, IntPtr environment, string cwd, ref StartupInfo startup, out ProcessInformation process);
            [DllImport("kernel32.dll", SetLastError = true)] internal static extern bool AssignProcessToJobObject(SafeKernelHandle job, IntPtr process);
            [DllImport("kernel32.dll", SetLastError = true)] internal static extern uint ResumeThread(IntPtr thread);
            [DllImport("kernel32.dll", SetLastError = true)] internal static extern bool TerminateProcess(IntPtr process, uint exitCode);
            [DllImport("kernel32.dll", SetLastError = true)] internal static extern bool GetExitCodeProcess(SafeKernelHandle process, out uint exitCode);
            [DllImport("kernel32.dll", SetLastError = true)] internal static extern bool TerminateJobObject(SafeKernelHandle job, uint exitCode);
            [DllImport("kernel32.dll", SetLastError = true)] internal static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
            [DllImport("kernel32.dll", SetLastError = true)] internal static extern uint WaitForSingleObject(SafeKernelHandle handle, uint milliseconds);
            [DllImport("kernel32.dll", SetLastError = true)] internal static extern bool QueryInformationJobObject(SafeKernelHandle job, int kind, IntPtr info, uint size, out uint returned);
            [DllImport("kernel32.dll", SetLastError = true)] internal static extern bool IsProcessInJob(IntPtr process, IntPtr job, out bool result);
            [DllImport("kernel32.dll")] internal static extern IntPtr GetCurrentProcess();
            [DllImport("kernel32.dll", SetLastError = true)] internal static extern bool CloseHandle(IntPtr handle);
            [DllImport("advapi32.dll", SetLastError = true)] internal static extern bool OpenProcessToken(IntPtr process, uint access, out IntPtr token);
            [DllImport("advapi32.dll", SetLastError = true)] internal static extern bool GetTokenInformation(IntPtr token, int kind, out int value, int size, out int returned);
        }
    }
}
