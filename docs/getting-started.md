# Getting started

**Evidence state: available; install verification is machine-dependent.**

What Changed is a per-user Windows desktop application. It records local
change evidence without an account or runtime network connection.

1. Download the release installer named in the release manifest.
2. Windows may show **Windows protected your PC — Unknown publisher** because
   the preview binary is unsigned. Select **More info → Run anyway** only when
   the checksum and source are trusted.
3. Finish the per-user install. No administrator prompt is expected.
4. Complete or skip the welcome tour. Collector preferences are held behind
   the local onboarding fence until you make that choice. Choose watched
   folders only when you want configuration evidence from those folders.
5. Wait for the Home screen to show the local monitoring state.

The tour applies the selected collectors before taking the first automatic
`First day` checkpoint. A failed save leaves the tour open and rolls back any
workers it started, so a partial onboarding choice cannot silently begin
monitoring. Reopening the tour from Settings keeps existing history but pauses
the selected collectors until completion again.

If WebView2 is missing, the installer may download its bootstrapper during
installation. That one install-time exception is separate from the runtime
no-network promise. See [deployment](deployment-guide.md).
