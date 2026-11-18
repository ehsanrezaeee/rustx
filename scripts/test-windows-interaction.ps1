# Exercise the extracted RP-007 counter through its own window and UIA tree.
param([Parameter(Mandatory)][string]$Executable)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class RustXWindowTest {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool PostMessageW(IntPtr hwnd, uint message, IntPtr key, IntPtr flags);
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y,
        int width, int height, uint flags);
}
'@

function Wait-For($description, [scriptblock]$probe) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    do {
        if ($app.HasExited) { throw "Counter exited while waiting for $description" }
        $value = & $probe
        if ($value) { return $value }
        Start-Sleep -Milliseconds 100
    } while ($timer.Elapsed.TotalSeconds -lt 15)
    $names = if ($window) {
        $window.FindAll([Windows.Automation.TreeScope]::Subtree,
            [Windows.Automation.Condition]::TrueCondition) | ForEach-Object {
                "$($_.Current.ControlType.ProgrammaticName) '$($_.Current.Name)'"
            }
    }
    throw "Timed out waiting for $description. Window tree: $($names -join ', ')"
}

function Find-Label([string]$name) {
    $condition = [Windows.Automation.PropertyCondition]::new(
        [Windows.Automation.AutomationElement]::NameProperty, $name)
    $window.FindFirst([Windows.Automation.TreeScope]::Descendants, $condition)
}

function Send-WindowKey([int]$key, [int]$scan) {
    # Post only to this process. Include the scan code, transition and previous-state bits.
    $down = 1 -bor ($scan -shl 16)
    $up = $down -bor [int]0xc0000000
    if (![RustXWindowTest]::PostMessageW($handle, 0x100, [IntPtr]$key, [IntPtr]$down) -or
        ![RustXWindowTest]::PostMessageW($handle, 0x101, [IntPtr]$key, [IntPtr]$up)) {
        throw "Could not post key $key to the counter"
    }
}

$exe = (Resolve-Path -LiteralPath $Executable).ProviderPath
$start = [Diagnostics.ProcessStartInfo]::new()
$start.FileName = $exe
$start.UseShellExecute = $false
$start.WorkingDirectory = [IO.Path]::GetTempPath()
$start.EnvironmentVariables['RUSTX_RENDERER'] = 'cpu'
$start.EnvironmentVariables.Remove('RUSTX_SMOKE_FRAMES')
$app = [Diagnostics.Process]::Start($start)
try {
    $handle = Wait-For 'counter window' {
        $app.Refresh()
        if ($app.MainWindowHandle -ne [IntPtr]::Zero) { $app.MainWindowHandle }
    }
    $window = [Windows.Automation.AutomationElement]::FromHandle($handle)
    $null = Wait-For 'Count: 0' { Find-Label 'Count: 0' }
    Send-WindowKey 0x09 0x0f # Tab to Increase
    Send-WindowKey 0x0d 0x1c # Enter
    $null = Wait-For 'Count: 1 after Tab and Enter' { Find-Label 'Count: 1' }

    $before = $window.Current.BoundingRectangle
    # NOMOVE | NOZORDER | NOACTIVATE
    if (![RustXWindowTest]::SetWindowPos($handle, [IntPtr]::Zero, 0, 0, 640, 480, 0x16)) {
        throw 'Window resize failed'
    }
    $after = Wait-For 'a changed window width' {
        $rect = $window.Current.BoundingRectangle
        if ($rect.Width -gt 0 -and $rect.Width -ne $before.Width) { $rect }
    }
    $null = Wait-For 'retained count after resize' { Find-Label 'Count: 1' }
    Send-WindowKey 0x20 0x39 # Space activates the still-focused button
    $null = Wait-For 'Count: 2 after Space' { Find-Label 'Count: 2' }
    if (!$app.CloseMainWindow() -or !$app.WaitForExit(10000) -or $app.ExitCode -ne 0) {
        throw 'Counter did not close normally'
    }
    [ordered]@{
        executableSha256 = (Get-FileHash -LiteralPath $exe).Hash.ToLowerInvariant()
        osVersion = [Environment]::OSVersion.Version.ToString()
        renderer = 'cpu'
        input = 'Win32 posted Tab/Enter/Space messages, UI Automation counter readback'
        counts = @(0, 1, 1, 2)
        windowBefore = @($before.Width, $before.Height)
        windowAfter = @($after.Width, $after.Height)
        exitCode = $app.ExitCode
    } | ConvertTo-Json
} finally {
    if (!$app.HasExited) { $app.Kill(); $app.WaitForExit() }
    $app.Dispose()
}
