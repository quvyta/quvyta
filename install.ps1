# Installs Quvyta apps with cargo, on Windows.
#
#   irm https://raw.githubusercontent.com/quvyta/quvyta/main/install.ps1 | iex
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/quvyta/quvyta/main/install.ps1))) code -Yes
#
# The second form passes arguments, which iex cannot. Run it with -Help for the options. Rust comes
# from rustup's own address, the applications from crates.io, and the only setting written is the
# user's own PATH, after you agree to the exact folder shown. The Visual Studio Build Tools, a
# system component, are installed only when they are missing and you say yes, at the console, to
# the one winget command shown; -Yes never says that yes for you, and Windows asks for permission
# itself.
#
# Everything lives in functions and runs from the last line, so a download cut short midway runs
# nothing at all. It works on Windows PowerShell 5.1 and PowerShell 7.

function Get-QuvytaApps {
    # The same names, crates and commands as install.sh; a test keeps the two in step.
    #
    # Soon marks a member that is not released yet: there is nothing on crates.io to install.
    # Naming it says so and installs nothing for it; it is left out of all and out of the picker's
    # numbers. Every member is out today; a new one that is not gets Soon = $true on its line here,
    # its name on the soon= line in install.sh, and Status::Soon in src/ecosystem.rs.
    @(
        [pscustomobject]@{ Name = 'framework'; Crate = 'quvyta-framework-showcase'; Command = 'qframe'; ArchOnly = $false; Soon = $false; About = 'the showcase of the framework every member is built on' }
        [pscustomobject]@{ Name = 'code'; Crate = 'quvyta-code'; Command = 'qcode'; ArchOnly = $false; Soon = $false; About = 'coding agents inside Podman or Docker containers' }
        [pscustomobject]@{ Name = 'focus'; Crate = 'quvyta-focus'; Command = 'qfocus'; ArchOnly = $false; Soon = $false; About = 'tracks what you focus on and where your time went' }
        [pscustomobject]@{ Name = 'packages'; Crate = 'quvyta-packages'; Command = 'qpac'; ArchOnly = $true; Soon = $false; About = 'a package manager for Arch Linux that shows every change first' }
        [pscustomobject]@{ Name = 'tools'; Crate = 'quvyta-tools'; Command = 'qtools'; ArchOnly = $true; Soon = $false; About = 'the settings Arch Linux users usually set up by hand, with undo' }
        [pscustomobject]@{ Name = 'quvyta'; Crate = 'quvyta'; Command = 'quvyta'; ArchOnly = $false; Soon = $false; About = 'installs, opens, updates and removes the Quvyta apps' }
        [pscustomobject]@{ Name = 'desk'; Crate = 'quvyta-desktop'; Command = 'qdesk'; ArchOnly = $false; Soon = $false; About = 'a desktop inside the terminal: windows, icons, a dock and a launcher' }
    )
}

# The members that can be installed today, which is what all and the picker's numbers offer.
function Get-QuvytaInstallable {
    @(Get-QuvytaApps | Where-Object { -not $_.Soon })
}

function Get-QuvytaMember {
    param([string]$Name)
    foreach ($member in Get-QuvytaApps) {
        if ($member.Name -eq $Name) { return $member }
    }
    return $null
}

function Write-QuvytaLine {
    [Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingWriteHost', '', Justification = 'An installer talks to the person at the console; its only output is text for them.')]
    param([string]$Text = '')
    Write-Host $Text
}

function Write-QuvytaUsage {
    Write-QuvytaLine @'
Installs Quvyta apps with cargo, on Windows.

Usage:
  irm https://raw.githubusercontent.com/quvyta/quvyta/main/install.ps1 | iex
  & ([scriptblock]::Create((irm https://raw.githubusercontent.com/quvyta/quvyta/main/install.ps1))) [options] [names]
  .\install.ps1 [options] [names]

Names (several may be given; none lets you choose):
  framework   quvyta-framework-showcase, command qframe
  code        quvyta-code, command qcode
  focus       quvyta-focus, command qfocus
  packages    quvyta-packages, command qpac; Arch Linux only, not installed on Windows
  tools       quvyta-tools, command qtools; Arch Linux only, not installed on Windows
  quvyta      quvyta, command quvyta
  desk        quvyta-desktop, command qdesk
  all         every one of the above that runs on Windows and is released

Options:
  -Yes        agree to every question: installing Rust with rustup, installing the chosen
              crates and adding cargo's folder to your user PATH (also --yes or -y)
  -Help       show this text (also --help or -h)

Without a console to ask on and without -Yes nothing is installed or written; the script only
says what it would do. When the Visual Studio C++ Build Tools that Rust needs are missing, it
shows the winget command that installs them and offers to run it. Only a yes typed at the
console runs it; -Yes does not, and the answer is no by default.
'@
}

# The one line a person sees for a member that is not out yet, in the words quvyta itself uses.
function Write-QuvytaSoon {
    param([string]$Name)
    $command = (Get-QuvytaMember $Name).Command
    Write-QuvytaLine "$command is not released yet, so it cannot be installed; quvyta's own list shows it as coming soon."
}

function Test-QuvytaWindows {
    [Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT
}

# Whether a question can be asked: not in a session started with -NonInteractive, not in a
# service, and not when the console's input comes from a file or a pipe.
function Test-QuvytaConsole {
    if (-not [Environment]::UserInteractive) { return $false }
    foreach ($argument in [Environment]::GetCommandLineArgs()) {
        if ($argument -match '^[-/]noni') { return $false }
    }
    if ($Host.Name -eq 'Windows PowerShell ISE Host') { return $true }
    try {
        if ([Console]::IsInputRedirected) { return $false }
    } catch {
        return $false
    }
    return $true
}

function Read-QuvytaAnswer {
    param([string]$Prompt)
    Read-Host -Prompt $Prompt
}

# Asks a yes-or-no question: 0 yes, 1 no, 2 cannot ask. Enter means yes.
function Confirm-QuvytaStep {
    param($State, [string]$Question)
    if ($State.Yes) { return 0 }
    if (-not $State.Console) { return 2 }
    $answer = Read-QuvytaAnswer "$Question (Y/n)"
    if ($null -eq $answer) { return 1 }
    if ($answer.Trim() -match '^(|y|yes)$') { return 0 }
    return 1
}

# Asks before a step that changes the system itself, such as installing the Build Tools: 0 yes,
# 1 no, 2 cannot ask. Only a yes typed at the console counts, so -Yes does not answer it and Enter
# means no.
function Confirm-QuvytaSystemStep {
    param($State, [string]$Question)
    if ($State.Yes -or -not $State.Console) { return 2 }
    $answer = Read-QuvytaAnswer "$Question (y/N)"
    if ($null -eq $answer) { return 1 }
    if ($answer.Trim() -match '^(y|yes)$') { return 0 }
    return 1
}

# Adds a name to the chosen list once, keeping the order.
function Add-QuvytaChoice {
    param($State, [string]$Name)
    if ($State.Chosen -notcontains $Name) { $State.Chosen += $Name }
}

# Reads the arguments. The PowerShell forms (-Yes, -Help) and those of install.sh (--yes, -y,
# --help, -h) mean the same. Returns 0, or 2 for an unknown name or option.
function Read-QuvytaCommandLine {
    param($State, [object[]]$Arguments)
    foreach ($argument in $Arguments) {
        $word = [string]$argument
        if ($word -in @('-Yes', '--yes', '-y')) { $State.Yes = $true; continue }
        if ($word -in @('-Help', '--help', '-h', '-?')) { $State.Help = $true; continue }
        if ($word -eq 'all') {
            foreach ($member in Get-QuvytaInstallable) { Add-QuvytaChoice $State $member.Name }
            continue
        }
        if ($word.StartsWith('-')) {
            Write-QuvytaLine "Unknown option: $word"
            Write-QuvytaLine 'Run with -Help to see the options.'
            return 2
        }
        $named = Get-QuvytaMember $word
        if ($named) {
            if ($named.Soon) {
                if ($State.Soon -notcontains $named.Name) { $State.Soon += $named.Name }
            } else {
                Add-QuvytaChoice $State $word
            }
        } else {
            Write-QuvytaLine "Unknown name: $word"
            Write-QuvytaLine "Known names: $((Get-QuvytaApps | ForEach-Object { $_.Name }) -join ' ') (or all)."
            return 2
        }
    }
    return 0
}

function Write-QuvytaApps {
    $number = 1
    foreach ($member in Get-QuvytaApps) {
        $about = $member.About
        # A member still to come has no number: the picker installs, and this cannot be installed.
        if ($member.Soon) {
            Write-QuvytaLine ('     {0,-10} {1,-8} {2}' -f $member.Name, $member.Command, "$about; coming soon, not released yet")
            continue
        }
        if ($member.ArchOnly) { $about = "$about; Arch Linux only, not for Windows" }
        Write-QuvytaLine ('  {0}  {1,-10} {2,-8} {3}' -f $number, $member.Name, $member.Command, $about)
        $number++
    }
}

# Turns the reply of the picker (numbers, names or all) into the chosen list. Returns whether
# every word was understood.
function Read-QuvytaPick {
    param($State, [string]$Reply)
    $installable = Get-QuvytaInstallable
    foreach ($word in @($Reply -split '[\s,]+' | Where-Object { $_ })) {
        if ($word -eq 'all') {
            foreach ($member in $installable) { Add-QuvytaChoice $State $member.Name }
        } elseif ($word -match '^[0-9]+$') {
            $number = [int]$word
            if ($number -lt 1 -or $number -gt $installable.Count) {
                Write-QuvytaLine "Unknown choice: $word"
                return $false
            }
            Add-QuvytaChoice $State $installable[$number - 1].Name
        } elseif (Get-QuvytaMember $word) {
            if ((Get-QuvytaMember $word).Soon) {
                Write-QuvytaSoon $word
            } else {
                Add-QuvytaChoice $State $word
            }
        } else {
            Write-QuvytaLine "Unknown choice: $word"
            return $false
        }
    }
    return $true
}

function Select-QuvytaMember {
    param($State)
    Write-QuvytaLine 'The Quvyta apps:'
    Write-QuvytaApps
    if (-not $State.Console) {
        Write-QuvytaLine ''
        Write-QuvytaLine 'There is no console to choose on. Name the members instead, for example:'
        Write-QuvytaLine '  & ([scriptblock]::Create((irm https://raw.githubusercontent.com/quvyta/quvyta/main/install.ps1))) code focus -Yes'
        return $false
    }
    while ($true) {
        $reply = Read-QuvytaAnswer 'Which ones? Numbers or names, separated by spaces, or all'
        if ([string]::IsNullOrWhiteSpace($reply)) {
            Write-QuvytaLine 'Nothing chosen, nothing installed.'
            return $false
        }
        $State.Chosen = @()
        if ((Read-QuvytaPick $State $reply) -and $State.Chosen.Count -gt 0) { return $true }
    }
}

# Leaves out the members that run on Arch Linux only, saying so for each.
function Select-QuvytaSupported {
    param($State)
    $kept = @()
    foreach ($name in $State.Chosen) {
        $member = Get-QuvytaMember $name
        if ($member.ArchOnly) {
            Write-QuvytaLine "Skipping $name ($($member.Command)): it runs on Arch Linux only."
        } else {
            $kept += $name
        }
    }
    $State.Chosen = $kept
    if ($kept.Count -eq 0) {
        Write-QuvytaLine 'Nothing left to install on Windows.'
        return $false
    }
    return $true
}

# Runs a program with its output shown on the console and returns its exit code. Programs such
# as cargo report progress on standard error; Windows PowerShell 5.1 turns that into errors when
# errors stop the script, so they are let through here.
function Invoke-QuvytaNative {
    param([string]$File, [string[]]$Arguments)
    $ErrorActionPreference = 'Continue'
    try {
        & $File @Arguments | Out-Host
        return $LASTEXITCODE
    } catch {
        Write-QuvytaLine "Could not run ${File}: $($_.Exception.Message)"
        return 1
    }
}

# Runs a program and returns its standard output as lines, or nothing when it cannot be run.
function Get-QuvytaNativeOutput {
    param([string]$File, [string[]]$Arguments)
    $ErrorActionPreference = 'Continue'
    try {
        @(& $File @Arguments 2>$null | ForEach-Object { [string]$_ })
    } catch {
        @()
    }
}

function Find-QuvytaProgram {
    param([string]$Name)
    $found = Get-Command -Name $Name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found) { return $found.Path }
    return $null
}

# Puts a folder in front of this session's PATH, so programs installed into it run by name here.
function Add-QuvytaSessionPath {
    param([string]$Folder)
    $env:PATH = $Folder + [IO.Path]::PathSeparator + $env:PATH
}

function Get-QuvytaRustupArch {
    $arch = $env:PROCESSOR_ARCHITEW6432
    if (-not $arch) { $arch = $env:PROCESSOR_ARCHITECTURE }
    switch ($arch) {
        'AMD64' { return 'x86_64' }
        'ARM64' { return 'aarch64' }
    }
    return $null
}

function Save-QuvytaDownload {
    param([string]$Url, [string]$Path)
    if (-not $Url.StartsWith('https://')) { throw "Refusing to download over plain http: $Url" }
    # Windows PowerShell 5.1 may still default to TLS 1.0, which the download server refuses.
    $protocols = [Net.ServicePointManager]::SecurityProtocol
    try {
        if ($PSVersionTable.PSVersion.Major -lt 6) {
            [Net.ServicePointManager]::SecurityProtocol = $protocols -bor [Net.SecurityProtocolType]::Tls12
        }
        $previousProgress = $ProgressPreference
        # The progress bar of Windows PowerShell 5.1 slows the download down many times over.
        $ProgressPreference = 'SilentlyContinue'
        try {
            Invoke-WebRequest -Uri $Url -OutFile $Path -UseBasicParsing
        } finally {
            $ProgressPreference = $previousProgress
        }
    } finally {
        [Net.ServicePointManager]::SecurityProtocol = $protocols
    }
}

function Install-QuvytaRust {
    param($State)
    if (Find-QuvytaProgram 'cargo.exe') { return $true }
    # rustup may have been installed earlier without its folder on PATH yet.
    if (Test-Path -LiteralPath (Join-Path $State.BinDir 'cargo.exe') -PathType Leaf) {
        Add-QuvytaSessionPath $State.BinDir
        return $true
    }
    $arch = Get-QuvytaRustupArch
    Write-QuvytaLine ''
    Write-QuvytaLine "cargo, Rust's package tool, is not installed. The Quvyta applications are built with it."
    if (-not $arch) {
        Write-QuvytaLine 'This processor is not one rustup-init.exe is offered for here. Install Rust from https://rustup.rs and run this script again.'
        return $false
    }
    $url = "https://static.rust-lang.org/rustup/dist/$arch-pc-windows-msvc/rustup-init.exe"
    Write-QuvytaLine "rustup, the official Rust installer, can install it for you into $($State.RustupHome) and $($State.CargoHome)."
    Write-QuvytaLine "It is downloaded from $url and does not need an administrator."
    switch (Confirm-QuvytaStep $State 'Install Rust with rustup now?') {
        0 { }
        1 {
            Write-QuvytaLine 'Rust was not installed. Install it from https://rustup.rs and run this script again.'
            return $false
        }
        default {
            Write-QuvytaLine 'There is no console to ask on, so nothing was installed. To install Rust yourself,'
            Write-QuvytaLine "download and run $url"
            Write-QuvytaLine 'then run this script again, or run it with -Yes to let it install Rust.'
            return $false
        }
    }
    $installer = Join-Path ([IO.Path]::GetTempPath()) ("rustup-init-" + [guid]::NewGuid().ToString('N') + '.exe')
    try {
        try {
            Save-QuvytaDownload $url $installer
        } catch {
            Write-QuvytaLine "rustup could not be downloaded: $($_.Exception.Message)"
            return $false
        }
        # Its own PATH change is turned off: the PATH step at the end asks before writing. With -y
        # rustup also leaves the Visual Studio Build Tools alone; they are checked after this.
        $code = Invoke-QuvytaNative $installer @('-y', '--no-modify-path', '--profile', 'minimal')
        if ($code -ne 0) {
            Write-QuvytaLine 'rustup could not install Rust.'
            return $false
        }
    } finally {
        Remove-Item -LiteralPath $installer -Force -ErrorAction SilentlyContinue
    }
    Add-QuvytaSessionPath $State.BinDir
    if (-not (Find-QuvytaProgram 'cargo.exe')) {
        Write-QuvytaLine "rustup finished but cargo is still not found in $($State.BinDir)."
        return $false
    }
    Write-QuvytaLine ''
    Write-QuvytaLine "Rust is installed. rustup's note about PATH is taken care of at the end."
    return $true
}

function Test-QuvytaRustVersion {
    param($State)
    $version = Get-QuvytaNativeOutput (Find-QuvytaProgram 'cargo.exe') @('--version') | Select-Object -First 1
    if (-not ($version -match '^cargo ([0-9]+)\.([0-9]+)')) { return $true }
    $major = [int]$Matches[1]
    $minor = [int]$Matches[2]
    if ($major -gt $State.MinMajor -or ($major -eq $State.MinMajor -and $minor -ge $State.MinMinor)) { return $true }
    Write-QuvytaLine ''
    Write-QuvytaLine "Rust $major.$minor is installed; the Quvyta applications need $($State.MinMajor).$($State.MinMinor) or later."
    $rustup = Find-QuvytaProgram 'rustup.exe'
    if ($rustup) {
        switch (Confirm-QuvytaStep $State 'Update it with rustup update stable?') {
            0 { if ((Invoke-QuvytaNative $rustup @('update', 'stable')) -eq 0) { return $true } }
            2 { Write-QuvytaLine 'There is no console to ask on. Run: rustup update stable' }
        }
    } else {
        Write-QuvytaLine 'Update Rust with the tool you installed it with, then run this script again.'
    }
    return $false
}

function Find-QuvytaVswhere {
    foreach ($root in @(${env:ProgramFiles(x86)}, $env:ProgramFiles)) {
        if ($root) {
            $candidate = Join-Path $root 'Microsoft Visual Studio\Installer\vswhere.exe'
            if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
        }
    }
    return Find-QuvytaProgram 'vswhere.exe'
}

# The MSVC toolchain links with the Visual Studio C++ Build Tools. A GNU toolchain brings its
# own linker, so the check is only made when Rust's default host is an MSVC one.
function Test-QuvytaLinker {
    param($State)
    $rustc = Find-QuvytaProgram 'rustc.exe'
    if ($rustc) {
        foreach ($line in Get-QuvytaNativeOutput $rustc @('-vV')) {
            if ($line -match '^host: .*-windows-gnu') { return $true }
        }
    }
    $arch = Get-QuvytaRustupArch
    $component = 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64'
    $extra = ''
    if ($arch -eq 'aarch64') {
        $component = 'Microsoft.VisualStudio.Component.VC.Tools.ARM64'
        $extra = ' --add Microsoft.VisualStudio.Component.VC.Tools.ARM64'
    }
    $vswhere = Find-QuvytaVswhere
    if ($vswhere) {
        $found = Get-QuvytaNativeOutput $vswhere @('-latest', '-products', '*', '-requires', $component, '-property', 'installationPath')
        if (@($found | Where-Object { $_.Trim() }).Count -gt 0) { return $true }
    }
    $override = "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended$extra"
    $arguments = @('install', '--id', 'Microsoft.VisualStudio.2022.BuildTools', '--exact', '--override', $override)
    $download = 'Or download them from https://visualstudio.microsoft.com/visual-cpp-build-tools/ and choose "Desktop development with C++".'
    Write-QuvytaLine ''
    Write-QuvytaLine 'Rust needs the Visual Studio C++ Build Tools to link programs, and they were not found.'
    $winget = Find-QuvytaProgram 'winget.exe'
    if (-not $winget -or $State.Yes -or -not $State.Console) {
        Write-QuvytaLine 'Install them with this command, then open a new terminal and run this script again:'
        Write-QuvytaLine "  winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --override `"$override`""
        Write-QuvytaLine $download
        return $false
    }
    Write-QuvytaLine 'This command installs them:'
    Write-QuvytaLine "  winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --override `"$override`""
    Write-QuvytaLine 'Windows will ask for your permission in a window of its own.'
    if ((Confirm-QuvytaSystemStep $State 'Run it now?') -ne 0) {
        Write-QuvytaLine 'Nothing was run. Install them, then open a new terminal and run this script again.'
        Write-QuvytaLine $download
        return $false
    }
    # The Visual Studio installer can still want a restart or a fresh session before its tools
    # are found, so the build starts again from a new terminal rather than here.
    if ((Invoke-QuvytaNative $winget $arguments) -eq 0) {
        Write-QuvytaLine 'The Build Tools are installed. Open a new terminal and run this script again.'
    } else {
        Write-QuvytaLine 'winget did not finish the install; its messages above say why.'
        Write-QuvytaLine $download
    }
    return $false
}

function Install-QuvytaChosen {
    param($State)
    Write-QuvytaLine ''
    Write-QuvytaLine "These will be built from crates.io and installed into $($State.BinDir):"
    foreach ($name in $State.Chosen) {
        $member = Get-QuvytaMember $name
        Write-QuvytaLine ('  {0,-27} command {1}' -f $member.Crate, $member.Command)
    }
    Write-QuvytaLine 'Building takes a few minutes for each.'
    switch (Confirm-QuvytaStep $State 'Install them now?') {
        0 { }
        1 {
            Write-QuvytaLine 'Nothing was installed.'
            return $false
        }
        default {
            Write-QuvytaLine 'There is no console to ask on, so nothing was installed. Run with -Yes, or yourself:'
            foreach ($name in $State.Chosen) {
                Write-QuvytaLine "  cargo install --locked $((Get-QuvytaMember $name).Crate)"
            }
            return $false
        }
    }
    $cargo = Find-QuvytaProgram 'cargo.exe'
    foreach ($name in $State.Chosen) {
        $crate = (Get-QuvytaMember $name).Crate
        Write-QuvytaLine ''
        Write-QuvytaLine "Installing $crate..."
        if ((Invoke-QuvytaNative $cargo @('install', '--locked', $crate)) -eq 0) {
            $State.Installed += $name
        } else {
            $State.Failed += $crate
        }
    }
    return $true
}

# The user's own PATH as it is stored, with entries such as %USERPROFILE%\bin left unexpanded,
# or $null when it is not a text value and must not be touched.
function Get-QuvytaUserPath {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment')
    if ($null -eq $key) { return '' }
    try {
        if ($key.GetValueNames() -notcontains 'Path') { return '' }
        $kind = $key.GetValueKind('Path')
        if ($kind -ne [Microsoft.Win32.RegistryValueKind]::String -and $kind -ne [Microsoft.Win32.RegistryValueKind]::ExpandString) {
            return $null
        }
        return [string]$key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    } finally {
        $key.Close()
    }
}

function Get-QuvytaMachinePath {
    [string][Environment]::GetEnvironmentVariable('Path', 'Machine')
}

# Stores the user's PATH as an expandable string, so entries such as %USERPROFILE%\bin keep
# working; [Environment]::SetEnvironmentVariable would store them expanded, as a plain string.
function Write-QuvytaUserPath {
    param([string]$Value)
    $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment')
    try {
        $key.SetValue('Path', $Value, [Microsoft.Win32.RegistryValueKind]::ExpandString)
    } finally {
        $key.Close()
    }
    Send-QuvytaSettingChange
}

# Tells Explorer and other programs that the environment changed, so terminals opened from them
# afterwards see the new PATH, as rustup does after changing it.
function Send-QuvytaSettingChange {
    try {
        if (-not ('QuvytaInstaller.Environment' -as [type])) {
            Add-Type -Namespace QuvytaInstaller -Name Environment -MemberDefinition @'
[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
'@
        }
        $result = [UIntPtr]::Zero
        # HWND_BROADCAST, WM_SETTINGCHANGE, SMTO_ABORTIFHUNG, five seconds, as rustup sends it.
        [void][QuvytaInstaller.Environment]::SendMessageTimeout([IntPtr]0xffff, 0x1A, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$result)
    } catch {
        Write-QuvytaLine 'Programs that are already running were not told about the change; signing out and in again shows it to all of them.'
    }
}

# Whether a list of folders holds the given one, comparing them as Windows does: expanded, without
# a trailing backslash, ignoring case.
function Test-QuvytaPathHas {
    param([string]$List, [string]$Separator, [string]$Folder)
    $want = [Environment]::ExpandEnvironmentVariables($Folder).TrimEnd('\', '/')
    foreach ($entry in $List.Split($Separator)) {
        $have = [Environment]::ExpandEnvironmentVariables($entry.Trim().Trim('"')).TrimEnd('\', '/')
        if ($have -and $have -eq $want) { return $true }
    }
    return $false
}

# Sets PathState: on-path (nothing to do), next-terminal (the stored PATH has the folder, so a new
# terminal finds the programs) or by-hand (the folder still has to be added).
function Add-QuvytaCargoToPath {
    param($State)
    $bin = $State.BinDir
    if (Test-QuvytaPathHas -List $State.StartPath -Separator ([IO.Path]::PathSeparator) -Folder $bin) {
        $State.PathState = 'on-path'
        return
    }
    $State.PathState = 'by-hand'
    Write-QuvytaLine ''
    Write-QuvytaLine "$bin, where cargo puts programs, is not on your PATH yet."
    $userPath = Get-QuvytaUserPath
    $inUser = Test-QuvytaPathHas -List ([string]$userPath) -Separator ';' -Folder $bin
    $inMachine = Test-QuvytaPathHas -List (Get-QuvytaMachinePath) -Separator ';' -Folder $bin
    if ($inUser -or $inMachine) {
        Write-QuvytaLine 'Your PATH setting already has it; a new terminal will pick it up.'
        $State.PathState = 'next-terminal'
        return
    }
    if ($null -eq $userPath) {
        Write-QuvytaLine 'Your PATH setting is not stored as text, so it is left alone. Add the folder yourself in'
        Write-QuvytaLine 'Settings, under "Edit environment variables for your account".'
        return
    }
    Write-QuvytaLine 'This folder would be added to the end of your user PATH setting:'
    Write-QuvytaLine "  $bin"
    switch (Confirm-QuvytaStep $State 'Add it?') {
        0 {
            $value = $bin
            if ($userPath.Trim(';')) { $value = $userPath.TrimEnd(';') + ';' + $bin }
            try {
                Write-QuvytaUserPath $value
            } catch {
                Write-QuvytaLine "Could not change your PATH setting: $($_.Exception.Message) Add the folder yourself."
                return
            }
            Add-QuvytaSessionPath $bin
            Write-QuvytaLine 'Added to your user PATH.'
            $State.PathState = 'next-terminal'
        }
        1 { Write-QuvytaLine 'Not added. Add the folder to PATH yourself to run the programs by name.' }
        default { Write-QuvytaLine 'There is no console to ask on, so your PATH was not changed. Add the folder yourself.' }
    }
}

function Write-QuvytaSummary {
    param($State)
    Write-QuvytaLine ''
    if ($State.Installed.Count -gt 0) {
        Write-QuvytaLine 'Installed:'
        foreach ($name in $State.Installed) {
            $member = Get-QuvytaMember $name
            Write-QuvytaLine ('  {0,-8} {1}' -f $member.Command, $member.About)
        }
    }
    if ($State.Failed.Count -gt 0) {
        Write-QuvytaLine "Could not install: $($State.Failed -join ' '). The messages above say why."
    }
    if ($State.Installed.Count -gt 0) {
        switch ($State.PathState) {
            'on-path' { Write-QuvytaLine 'Run a command above by its name to start it.' }
            'next-terminal' { Write-QuvytaLine 'Open a new terminal, then run a command above by its name.' }
            default {
                $first = (Get-QuvytaMember $State.Installed[0]).Command
                Write-QuvytaLine "Until the folder is on PATH, start them by their full path, such as $(Join-Path $State.BinDir "$first.exe")."
            }
        }
    }
}

# PowerShell also runs on Linux and macOS, where install.sh does this installer's work. It is
# offered to run right here, with the same names and the same -Yes.
function Invoke-QuvytaUnixInstaller {
    param($State, [object[]]$Arguments)
    $url = 'https://raw.githubusercontent.com/quvyta/quvyta/main/install.sh'
    $forward = @()
    foreach ($argument in $Arguments) {
        $word = [string]$argument
        if ($word -in @('-Yes', '--yes', '-y')) { $word = '--yes' }
        if ($forward -notcontains $word) { $forward += $word }
    }
    $shown = "curl -fsSL $url | sh"
    if ($forward.Count -gt 0) { $shown = "$shown -s -- $($forward -join ' ')" }
    Write-QuvytaLine 'This installer is for Windows. On Linux and macOS use:'
    Write-QuvytaLine "  $shown"
    $sh = Find-QuvytaProgram 'sh'
    if (-not $sh -or -not (Find-QuvytaProgram 'curl')) { return 1 }
    switch (Confirm-QuvytaStep $State 'Run it now?') {
        0 { }
        1 {
            Write-QuvytaLine 'Nothing was run.'
            return 1
        }
        default { return 1 }
    }
    # sh keeps the address and the words apart, so none of them is read as shell code.
    return Invoke-QuvytaNative $sh (@('-c', 'curl -fsSL $0 | sh -s -- $@', $url) + $forward)
}

# Runs the whole installer and returns its exit code: 0 done, 1 not done or a member failed,
# 2 an unknown name or option.
function Invoke-QuvytaInstall {
    param([object[]]$Arguments)
    $cargoHome = $env:CARGO_HOME
    if (-not $cargoHome) { $cargoHome = Join-Path $HOME '.cargo' }
    $rustupHome = $env:RUSTUP_HOME
    if (-not $rustupHome) { $rustupHome = Join-Path $HOME '.rustup' }
    $State = @{
        Yes = $false
        Help = $false
        Chosen = @()
        Soon = @()
        Installed = @()
        Failed = @()
        CargoHome = $cargoHome
        RustupHome = $rustupHome
        BinDir = Join-Path $cargoHome 'bin'
        StartPath = [string]$env:PATH
        PathState = 'on-path'
        Console = $false
        # Every member published crates for, at least this Rust.
        MinMajor = 1
        MinMinor = 95
    }

    if ((Read-QuvytaCommandLine $State $Arguments) -ne 0) { return 2 }
    if ($State.Help) {
        Write-QuvytaUsage
        return 0
    }
    $State.Console = Test-QuvytaConsole
    if (-not (Test-QuvytaWindows)) { return Invoke-QuvytaUnixInstaller $State $Arguments }

    if ($State.Soon.Count -gt 0) {
        foreach ($name in $State.Soon) { Write-QuvytaSoon $name }
        # Named on its own, nothing is left to install; ending here also keeps the picker away.
        if ($State.Chosen.Count -eq 0) { return 1 }
    }
    if ($State.Chosen.Count -eq 0) {
        if (-not (Select-QuvytaMember $State)) { return 1 }
    }
    if (-not (Select-QuvytaSupported $State)) { return 1 }
    if (-not (Install-QuvytaRust $State)) { return 1 }
    if (-not (Test-QuvytaRustVersion $State)) { return 1 }
    if (-not (Test-QuvytaLinker $State)) { return 1 }
    if (-not (Install-QuvytaChosen $State)) { return 1 }
    if ($State.Installed.Count -gt 0) { Add-QuvytaCargoToPath $State }
    Write-QuvytaSummary $State
    if ($State.Failed.Count -gt 0) { return 1 }
    return 0
}

# The entry point. Strict mode and stopping on errors stay inside these functions, so the session
# that ran `irm ... | iex` keeps its own settings. The exit code goes to $LASTEXITCODE; only a run
# from the file itself ends with exit, since exit would close the window of an iex run.
function Invoke-QuvytaInstaller {
    param([object[]]$Arguments, [string]$ScriptFile)
    Set-StrictMode -Version Latest
    $ErrorActionPreference = 'Stop'
    try {
        $code = Invoke-QuvytaInstall $Arguments
    } catch {
        Write-QuvytaLine "The installer stopped: $($_.Exception.Message)"
        $code = 1
    }
    $global:LASTEXITCODE = $code
    if ($ScriptFile) { exit $code }
}

# Dot-sourcing only defines the functions, which is how the tests load them.
if ($MyInvocation.InvocationName -ne '.') { Invoke-QuvytaInstaller -Arguments $args -ScriptFile ({}.File) }
