# Drive only the native picker owned by the isolated test backend; never send global keystrokes.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$deadline = (Get-Date).AddSeconds(20)
$dialog = $null
while (-not $dialog -and (Get-Date) -lt $deadline) {
    $children = @(Get-CimInstance Win32_Process | Where-Object { $_.ParentProcessId -eq [int]$env:TETHER_PICK_PARENT -and $_.Name -eq 'powershell.exe' })
    foreach ($child in $children) {
        $condition = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, [int]$child.ProcessId)
        $dialog = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $condition)
        if ($dialog) { break }
    }
    if (-not $dialog) { Start-Sleep -Milliseconds 100 }
}
if (-not $dialog) { throw 'Isolated native picker did not appear' }
function ById($id) {
    $condition = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::AutomationIdProperty, $id)
    $class = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ClassNameProperty, 'Button')
    $both = New-Object System.Windows.Automation.AndCondition($condition,$class)
    $dialog.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $both)
}
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PickerNative {
 [DllImport("user32.dll", EntryPoint="SendMessageW", CharSet=CharSet.Unicode)]
 public static extern IntPtr Text(IntPtr window,uint message,IntPtr wParam,string value);
 [DllImport("user32.dll", EntryPoint="SendMessageW")]
 public static extern IntPtr Click(IntPtr window,uint message,IntPtr wParam,IntPtr lParam);
}
'@
if ($env:TETHER_PICK_CANCEL -eq '1') {
    $button = ById '2'
} else {
    $id = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::AutomationIdProperty, '1148')
    $class = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ClassNameProperty, 'Edit')
    $condition = New-Object System.Windows.Automation.AndCondition($id,$class)
    $edit = $dialog.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$condition)
    if (-not $edit -or -not $edit.Current.NativeWindowHandle) { throw 'Native filename control not found' }
    [PickerNative]::Text([IntPtr]$edit.Current.NativeWindowHandle,0x000C,[IntPtr]::Zero,$env:TETHER_PICK_PATH) | Out-Null
    $button = ById '1'
}
if (-not $button -or -not $button.Current.NativeWindowHandle) { throw 'Picker action not found' }
$command = if ($env:TETHER_PICK_CANCEL -eq '1') { 2 } else { 1 }
[PickerNative]::Click([IntPtr]$dialog.Current.NativeWindowHandle,0x0111,[IntPtr]$command,[IntPtr]$button.Current.NativeWindowHandle) | Out-Null
