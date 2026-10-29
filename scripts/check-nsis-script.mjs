import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve('.');
const scriptPath = path.resolve(
  process.argv[2] ?? path.join(root, 'src-tauri', 'target', 'release', 'nsis', 'x64', 'installer.nsi'),
);

if (!fs.existsSync(scriptPath)) {
  throw new Error(`NSIS script is missing: ${scriptPath}`);
}

const source = fs.readFileSync(scriptPath, 'utf8');
const config = JSON.parse(fs.readFileSync(path.join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'));
const hooksRelative = config.bundle.windows.nsis.installerHooks;
if (hooksRelative !== 'nsis/installer-hooks.nsh') {
  throw new Error('NSIS installer must use the reviewed desktop-shortcut hooks');
}
const hooksPath = path.join(root, 'src-tauri', hooksRelative);
const hooks = fs.readFileSync(hooksPath, 'utf8');
if (!source.includes(`!include "${hooksPath.replaceAll('/', '\\')}"`)) {
  throw new Error('generated NSIS script does not include the configured hook file');
}
for (const marker of [
  'MUI_FINISHPAGE_SHOWREADME_NOTCHECKED',
  'NSIS_HOOK_PREINSTALL',
  'NSIS_HOOK_POSTINSTALL',
  'DesktopShortcutExisted',
  'DesktopShortcutBackup',
  'DesktopShortcutBackupFile',
  'DesktopShortcutOriginalNoShortcutMode',
  'CreateDirectory "$DesktopShortcutBackup"',
  'StrCpy $NoShortcutMode 1',
  'Call CreateOrUpdateStartMenuShortcut',
  'CopyFiles /SILENT',
  '/DESKTOPSHORTCUT',
  'Delete "$DESKTOP\\${PRODUCTNAME}.lnk"',
]) {
  if (!hooks.includes(marker)) throw new Error(`NSIS shortcut hook is missing ${marker}`);
}
for (const [key, define] of [
  ['installerIcon', 'INSTALLERICON'],
  ['uninstallerIcon', 'UNINSTALLERICON'],
  ['headerImage', 'HEADERIMAGE'],
  ['uninstallerHeaderImage', 'UNINSTALLERHEADERIMAGE'],
  ['sidebarImage', 'SIDEBARIMAGE'],
]) {
  const configured = config.bundle.windows.nsis[key];
  if (typeof configured !== 'string' || !configured) throw new Error(`NSIS ${key} is not configured`);
  const input = path.resolve(root, 'src-tauri', configured);
  if (!fs.existsSync(input)) throw new Error(`NSIS ${key} asset is missing: ${input}`);
  const match = source.match(new RegExp(`^!define ${define} "([^"]*)"`, 'm'));
  const bundled = match?.[1];
  if (!bundled || !fs.existsSync(bundled)) throw new Error(`generated NSIS ${define} does not name a bundled asset`);
  if (!fs.readFileSync(input).equals(fs.readFileSync(bundled))) {
    throw new Error(`generated NSIS ${define} differs from the configured brand asset`);
  }
}
const required = [
  ['per-user execution level', 'RequestExecutionLevel user'],
  ['start-menu shortcut creation', 'CreateOrUpdateStartMenuShortcut'],
  ['desktop shortcut creation', 'CreateOrUpdateDesktopShortcut'],
  ['uninstall section', 'Section Uninstall'],
  ['history checkbox variable', 'Var DeleteAppDataCheckbox'],
  ['history checkbox label', '$(deleteAppData)'],
  ['history opt-in guard', 'Delete app data if the checkbox is selected'],
  ['roaming data deletion', 'RmDir /r "$APPDATA\\${BUNDLEID}"'],
  ['local data deletion', 'RmDir /r "$LOCALAPPDATA\\${BUNDLEID}"'],
];

for (const [label, marker] of required) {
  if (!source.includes(marker)) {
    throw new Error(`NSIS script is missing ${label}: ${marker}`);
  }
}

const uninstallStart = source.indexOf('Section Uninstall');
const uninstallEnd = source.indexOf('SectionEnd', uninstallStart);
if (uninstallStart < 0 || uninstallEnd < 0) {
  throw new Error('NSIS uninstall section is missing or incomplete');
}
const uninstallCommands = source
  .slice(uninstallStart, uninstallEnd)
  .split(/\r?\n/)
  .map((line) => line.replace(/^\s*;.*$/, ''))
  .join('\n');
const optInBlock = uninstallCommands.match(
  /^\s*\$\{If\}\s+\$DeleteAppDataCheckboxState\s*=\s*1\s*\n\s*\$\{AndIf\}\s+\$UpdateMode\s*<>\s*1\s*\n(?<body>[\s\S]*?)^\s*\$\{EndIf\}/m,
);
if (!optInBlock) {
  throw new Error('uninstall app-data deletion is missing the checkbox and non-update guard');
}
for (const directory of ['APPDATA', 'LOCALAPPDATA']) {
  const deletion = 'RmDir /r "$' + directory + '\\${BUNDLEID}"';
  if (!optInBlock.groups.body.includes(deletion)) {
    throw new Error(`${directory} deletion is outside the uninstall opt-in guard`);
  }
  if (uninstallCommands.split(deletion).length !== 2) {
    throw new Error(`${directory} deletion must occur exactly once in the uninstall section`);
  }
}

console.log(`check-nsis-script: PASS (${path.relative(root, scriptPath).replaceAll('\\', '/')})`);
