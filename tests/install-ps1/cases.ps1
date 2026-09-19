# The cases of install.ps1, run by run.sh inside a PowerShell container on Linux.
#
# The installer's functions are loaded by dot-sourcing it, which only defines them. The functions
# that need Windows itself (the registry, the settings broadcast, the Windows check, the console
# check and Read-Host) are then replaced by versions that work on what each case sets up; every
# other function runs as shipped. cargo, rustup, rustc and vswhere are small shell scripts named
# like the Windows programs, which Linux runs whatever their name; each writes what it was asked
# into a log, so a case can check that nothing was installed.

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$script:Source = '/src/install.ps1'
$script:Failures = 0
$script:CaseName = ''
$script:Work = Join-Path ([IO.Path]::GetTempPath()) ('install-ps1-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $script:Work | Out-Null
$script:BasePath = $env:PATH

. $script:Source

# --- The parts of Windows the cases stand in for

function Write-QuvytaLine {
    param([string]$Text = '')
    $script:Out.Add($Text)
}

function Test-QuvytaWindows { $true }

function Test-QuvytaConsole { $script:Console }

function Read-QuvytaAnswer {
    param([string]$Prompt)
    $script:Out.Add("? $Prompt")
    if ($script:Answers.Count -eq 0) { return $null }
    $script:Answers.Dequeue()
}

function Get-QuvytaUserPath { $script:UserPath }

function Get-QuvytaMachinePath { $script:MachinePath }

function Write-QuvytaUserPath {
    param([string]$Value)
    $script:UserPath = $Value
    $script:PathWrites++
}

# The download of rustup-init.exe: the "installer" puts the cargo stand-in where rustup would.
function Save-QuvytaDownload {
    param([string]$Url, [string]$Path)
    Add-Content -LiteralPath $script:Log -Value "download $Url"
    Set-Content -LiteralPath $Path -Value @"
#!/bin/sh
echo "rustup-init `$*" >>"$($script:Log)"
mkdir -p "`$CARGO_HOME/bin"
cp "$($script:Stubs)/cargo.real" "`$CARGO_HOME/bin/cargo.exe"
"@
    & chmod +x $Path
}

# --- Setting up a case

function Write-Stub {
    param([string]$Name, [string]$Body)
    $path = Join-Path $script:Stubs $Name
    Set-Content -LiteralPath $path -Value ("#!/bin/sh`n" + $Body)
    & chmod +x $path
}

function New-Case {
    param([string]$Name)
    $script:CaseName = $Name
    $script:Root = Join-Path $script:Work $Name
    $script:Stubs = Join-Path $script:Root 'stubs'
    $script:Log = Join-Path $script:Root 'log'
    New-Item -ItemType Directory -Path $script:Stubs | Out-Null
    New-Item -ItemType File -Path $script:Log | Out-Null
    $env:CARGO_HOME = Join-Path $script:Root 'cargo'
    $env:RUSTUP_HOME = Join-Path $script:Root 'rustup'
    $env:FAKE_CARGO_VERSION = '1.95.0'
    $env:PROCESSOR_ARCHITECTURE = 'AMD64'
    $env:PROCESSOR_ARCHITEW6432 = $null
    $env:ProgramFiles = $null
    [Environment]::SetEnvironmentVariable('ProgramFiles(x86)', (Join-Path $script:Root 'pf86'))
    $env:PATH = $script:Stubs + [IO.Path]::PathSeparator + $script:BasePath

    Write-Stub 'cargo.exe' @"
echo "cargo `$*" >>"$($script:Log)"
case `$1 in
    --version) echo "cargo `${FAKE_CARGO_VERSION} (stub)" ;;
    install)
        case `$3 in *fail*|`$FAKE_FAIL) exit 101 ;; esac
        mkdir -p "`$CARGO_HOME/bin" && : >"`$CARGO_HOME/bin/`$3.exe"
        ;;
esac
"@
    Copy-Item (Join-Path $script:Stubs 'cargo.exe') (Join-Path $script:Stubs 'cargo.real')
    Write-Stub 'rustup.exe' "echo `"rustup `$*`" >>`"$($script:Log)`""
    # The Build Tools are installed unless a case takes vswhere away or makes it find nothing.
    $installer = Join-Path $script:Root 'pf86/Microsoft Visual Studio/Installer'
    New-Item -ItemType Directory -Path $installer | Out-Null
    Set-Content -LiteralPath (Join-Path $installer 'vswhere.exe') -Value @"
#!/bin/sh
echo "vswhere `$*" >>"$($script:Log)"
echo 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools'
"@
    & chmod +x (Join-Path $installer 'vswhere.exe')

    $env:FAKE_FAIL = 'none'
    $script:Out = [System.Collections.Generic.List[string]]::new()
    $script:Console = $false
    $script:Answers = [System.Collections.Generic.Queue[string]]::new()
    $script:UserPath = 'C:\Windows\System32;%USERPROFILE%\AppData\Local\Microsoft\WindowsApps'
    $script:MachinePath = 'C:\Windows\System32'
    $script:PathWrites = 0
}

function Invoke-Case {
    param([object[]]$Arguments = @())
    $script:Status = Invoke-QuvytaInstall $Arguments
}

function Set-Answers {
    param([string[]]$Lines)
    $script:Console = $true
    foreach ($line in $Lines) { $script:Answers.Enqueue($line) }
}

# --- Checks

function Fail {
    param([string]$Message)
    Write-Host "FAIL $($script:CaseName): $Message"
    Write-Host '--- output'
    $script:Out | ForEach-Object { Write-Host $_ }
    Write-Host '--- log'
    Get-Content -LiteralPath $script:Log | ForEach-Object { Write-Host $_ }
    Write-Host '---'
    $script:Failures++
}

function Assert-Status {
    param([int]$Expected)
    if ($script:Status -ne $Expected) { Fail "status $($script:Status), expected $Expected" }
}

function Assert-Output {
    param([string]$Text)
    foreach ($line in $script:Out) {
        if ($line.Contains($Text)) { return }
    }
    Fail "output lacks: $Text"
}

function Assert-NoOutput {
    param([string]$Text)
    foreach ($line in $script:Out) {
        if ($line.Contains($Text)) {
            Fail "output should not have: $Text"
            return
        }
    }
}

function Get-LogLines { @(Get-Content -LiteralPath $script:Log) }

function Assert-Logged {
    param([string]$Line)
    if ((Get-LogLines) -notcontains $Line) { Fail "not run: $Line" }
}

# For lines whose start the container may change: pwsh on Linux expands a lone * given to a
# program, which Windows does not.
function Assert-LoggedPart {
    param([string]$Text)
    foreach ($line in Get-LogLines) {
        if ($line.Contains($Text)) { return }
    }
    Fail "not run: ...$Text"
}

function Assert-NotLogged {
    param([string]$Text)
    foreach ($line in Get-LogLines) {
        if ($line.Contains($Text)) {
            Fail "should not have run: $Text"
            return
        }
    }
}

function Assert-Equal {
    param($Actual, $Expected, [string]$What)
    if ($Actual -ne $Expected) { Fail "$What is '$Actual', expected '$Expected'" }
}

# Nothing installed, no PATH written, no Rust fetched.
function Assert-NothingDone {
    Assert-NotLogged 'cargo install'
    Assert-NotLogged 'download'
    Assert-Equal $script:PathWrites 0 'the number of PATH writes'
    if (Test-Path -LiteralPath $env:CARGO_HOME) { Fail 'CARGO_HOME was created' }
}

$bin = { Join-Path $env:CARGO_HOME 'bin' }

# --- The script as written

New-Case 'grammar-of-windows-powershell-5.1'
$tokens = $null
$errors = $null
[void][System.Management.Automation.Language.Parser]::ParseFile($script:Source, [ref]$tokens, [ref]$errors)
if ($errors.Count -gt 0) { Fail "parse errors: $($errors -join '; ')" }
$newer = @('QuestionQuestion', 'QuestionQuestionEquals', 'QuestionDot', 'QuestionLBracket', 'AndAnd', 'OrOr', 'QuestionMark')
foreach ($token in $tokens) {
    if ($newer -contains [string]$token.Kind) { Fail "PowerShell 7 only syntax at line $($token.Extent.StartLineNumber): $($token.Text)" }
}
$bytes = [IO.File]::ReadAllBytes($script:Source)
foreach ($byte in $bytes) {
    if ($byte -gt 127) {
        Fail 'non-ASCII text, which Windows PowerShell 5.1 misreads in a file without a byte order mark'
        break
    }
}
if ((Get-Content -LiteralPath $script:Source | Select-Object -Last 1) -notmatch '^if \(\$MyInvocation\.InvocationName -ne ''\.''\) \{ Invoke-QuvytaInstaller ') {
    Fail 'the last line does not start the installer'
}

# --- Arguments

New-Case 'help'
Invoke-Case @('-Help')
Assert-Status 0
Assert-Output 'quvyta-packages, command qpac; Arch Linux only'
Assert-Output '-Yes'
Assert-NothingDone

foreach ($form in @('--help', '-h', '-help')) {
    New-Case "help-$form"
    Invoke-Case @($form)
    Assert-Status 0
    Assert-Output 'Usage:'
}

New-Case 'unknown-name'
Invoke-Case @('code', 'nonsense')
Assert-Status 2
Assert-Output 'Unknown name: nonsense'
Assert-NotLogged 'cargo'

New-Case 'unknown-option'
Invoke-Case @('--force', 'code')
Assert-Status 2
Assert-Output 'Unknown option: --force'
Assert-NotLogged 'cargo'

foreach ($form in @('-Yes', '--yes', '-y', '-yes')) {
    New-Case "yes-$form"
    Invoke-Case @('code', $form)
    Assert-Status 0
    Assert-Logged 'cargo install --locked quvyta-code'
}

New-Case 'names-once-in-order'
Invoke-Case @('-Yes', 'focus', 'code', 'focus')
Assert-Status 0
$installs = @(Get-LogLines | Where-Object { $_ -like 'cargo install*' })
Assert-Equal ($installs -join ',') 'cargo install --locked quvyta-focus,cargo install --locked quvyta-code' 'the installs'

New-Case 'all-leaves-out-arch-only'
Invoke-Case @('-Yes', 'all')
Assert-Status 0
foreach ($crate in @('quvyta-framework-showcase', 'quvyta-code', 'quvyta-focus', 'quvyta')) {
    Assert-Logged "cargo install --locked $crate"
}
Assert-NotLogged 'quvyta-packages'
Assert-NotLogged 'quvyta-tools'
Assert-Output 'Skipping packages (qpac): it runs on Arch Linux only.'
Assert-Output 'Skipping tools (qtools): it runs on Arch Linux only.'

New-Case 'arch-only-alone'
Invoke-Case @('-Yes', 'tools')
Assert-Status 1
Assert-Output 'Nothing left to install on Windows.'
Assert-NotLogged 'cargo'
Assert-NothingDone

# --- Without a console

New-Case 'no-console-names'
Invoke-Case @('code')
Assert-Status 1
Assert-Output 'cargo install --locked quvyta-code'
Assert-Output 'There is no console to ask on'
Assert-NothingDone

New-Case 'no-console-no-names'
Invoke-Case @()
Assert-Status 1
Assert-Output 'There is no console to choose on'
Assert-Output '  4  packages   qpac     a package manager for Arch Linux that shows every change first; Arch Linux only, not for Windows'
Assert-NotLogged 'cargo'
Assert-NothingDone

New-Case 'no-console-no-cargo'
Remove-Item (Join-Path $script:Stubs 'cargo.exe')
Invoke-Case @('code')
Assert-Status 1
Assert-Output 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe'
Assert-Output 'There is no console to ask on, so nothing was installed.'
Assert-NothingDone

New-Case 'no-console-with-yes-writes-path'
$script:Console = $false
Invoke-Case @('-Yes', 'code')
Assert-Status 0
Assert-Equal $script:PathWrites 1 'the number of PATH writes'

# --- On a console

New-Case 'pick-numbers-and-agree'
Set-Answers @('2 3', 'y', 'y')
Invoke-Case @()
Assert-Status 0
Assert-Logged 'cargo install --locked quvyta-code'
Assert-Logged 'cargo install --locked quvyta-focus'
Assert-NotLogged 'quvyta-framework'
Assert-Equal $script:PathWrites 1 'the number of PATH writes'
Assert-Output 'Open a new terminal'

New-Case 'pick-names'
Set-Answers @('quvyta framework', '', 'n')
Invoke-Case @()
Assert-Status 0
Assert-Logged 'cargo install --locked quvyta'
Assert-Logged 'cargo install --locked quvyta-framework-showcase'
Assert-Equal $script:PathWrites 0 'the number of PATH writes'
Assert-Output 'Not added.'
Assert-Output "start them by their full path, such as $(Join-Path (& $bin) 'quvyta.exe')"

New-Case 'pick-bad-then-good'
Set-Answers @('9', 'code', 'y', 'y')
Invoke-Case @()
Assert-Status 0
Assert-Output 'Unknown choice: 9'
Assert-Logged 'cargo install --locked quvyta-code'

New-Case 'pick-empty'
Set-Answers @('')
Invoke-Case @()
Assert-Status 1
Assert-Output 'Nothing chosen, nothing installed.'
Assert-NotLogged 'cargo'
Assert-NothingDone

New-Case 'pick-arch-only'
Set-Answers @('4 1', 'y', 'y')
Invoke-Case @()
Assert-Status 0
Assert-Output 'Skipping packages (qpac)'
Assert-Logged 'cargo install --locked quvyta-framework-showcase'
Assert-NotLogged 'quvyta-packages'

New-Case 'decline-install'
Set-Answers @('n')
Invoke-Case @('code')
Assert-Status 1
Assert-Output 'Nothing was installed.'
Assert-NothingDone

# --- Rust itself

New-Case 'cargo-missing-rustup-offered'
Remove-Item (Join-Path $script:Stubs 'cargo.exe')
Set-Answers @('y', 'y', 'y')
Invoke-Case @('focus')
Assert-Status 0
Assert-Output 'Install Rust with rustup now?'
Assert-Logged 'download https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe'
Assert-Logged 'rustup-init -y --no-modify-path --profile minimal'
Assert-Output 'Rust is installed.'
Assert-Logged 'cargo install --locked quvyta-focus'
$left = @(Get-ChildItem -LiteralPath ([IO.Path]::GetTempPath()) -Filter 'rustup-init-*.exe')
Assert-Equal $left.Count 0 'the number of rustup-init files left behind'

New-Case 'cargo-missing-rustup-declined'
Remove-Item (Join-Path $script:Stubs 'cargo.exe')
Set-Answers @('n')
Invoke-Case @('focus')
Assert-Status 1
Assert-Output 'Rust was not installed.'
Assert-NothingDone

New-Case 'cargo-missing-arm64'
Remove-Item (Join-Path $script:Stubs 'cargo.exe')
$env:PROCESSOR_ARCHITECTURE = 'ARM64'
Invoke-Case @('-Yes', 'code')
Assert-Logged 'download https://static.rust-lang.org/rustup/dist/aarch64-pc-windows-msvc/rustup-init.exe'
Assert-LoggedPart '-requires Microsoft.VisualStudio.Component.VC.Tools.ARM64 -property installationPath'
Assert-Status 0

New-Case 'cargo-missing-32-bit-session-on-64-bit-windows'
Remove-Item (Join-Path $script:Stubs 'cargo.exe')
$env:PROCESSOR_ARCHITECTURE = 'x86'
$env:PROCESSOR_ARCHITEW6432 = 'AMD64'
Invoke-Case @('-Yes', 'code')
Assert-Logged 'download https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe'
Assert-Status 0

New-Case 'cargo-in-cargo-home-but-not-on-path'
Remove-Item (Join-Path $script:Stubs 'cargo.exe')
New-Item -ItemType Directory -Path (& $bin) | Out-Null
Copy-Item (Join-Path $script:Stubs 'cargo.real') (Join-Path (& $bin) 'cargo.exe')
Invoke-Case @('-Yes', 'code')
Assert-Status 0
Assert-NotLogged 'download'
Assert-Logged 'cargo install --locked quvyta-code'

New-Case 'rust-too-old-updated'
$env:FAKE_CARGO_VERSION = '1.80.2'
Invoke-Case @('-Yes', 'code')
Assert-Output 'Rust 1.80 is installed; the Quvyta applications need 1.95 or later.'
Assert-Logged 'rustup update stable'
Assert-Status 0

New-Case 'rust-too-old-no-console'
$env:FAKE_CARGO_VERSION = '1.80.2'
Invoke-Case @('code')
Assert-Status 1
Assert-Output 'Run: rustup update stable'
Assert-NotLogged 'rustup update'
Assert-NothingDone

New-Case 'rust-too-old-without-rustup'
$env:FAKE_CARGO_VERSION = '1.80.2'
Remove-Item (Join-Path $script:Stubs 'rustup.exe')
Invoke-Case @('-Yes', 'code')
Assert-Status 1
Assert-Output 'Update Rust with the tool you installed it with'
Assert-NotLogged 'cargo install'

# --- The Visual Studio Build Tools

New-Case 'build-tools-missing'
Remove-Item -Recurse (Join-Path $script:Root 'pf86')
Invoke-Case @('-Yes', 'code')
Assert-Status 1
Assert-Output 'Rust needs the Visual Studio C++ Build Tools'
Assert-Output '  winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"'
Assert-NotLogged 'cargo install'
Assert-NotLogged 'winget'
Assert-Equal $script:PathWrites 0 'the number of PATH writes'

New-Case 'build-tools-missing-vswhere-finds-nothing'
Write-Stub 'vswhere-empty' ''
Copy-Item -Force (Join-Path $script:Stubs 'vswhere-empty') (Join-Path $script:Root 'pf86/Microsoft Visual Studio/Installer/vswhere.exe')
$env:PROCESSOR_ARCHITECTURE = 'ARM64'
Invoke-Case @('-Yes', 'code')
Assert-Status 1
Assert-Output '--includeRecommended --add Microsoft.VisualStudio.Component.VC.Tools.ARM64"'
Assert-NotLogged 'cargo install'

New-Case 'build-tools-vswhere-on-path'
Move-Item (Join-Path $script:Root 'pf86/Microsoft Visual Studio/Installer/vswhere.exe') (Join-Path $script:Stubs 'vswhere.exe')
Invoke-Case @('-Yes', 'code')
Assert-Status 0
Assert-LoggedPart '-requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath'

New-Case 'build-tools-not-needed-for-gnu'
Remove-Item -Recurse (Join-Path $script:Root 'pf86')
Write-Stub 'rustc.exe' "printf 'rustc 1.95.0\nhost: x86_64-pc-windows-gnu\n'"
Invoke-Case @('-Yes', 'code')
Assert-Status 0
Assert-Logged 'cargo install --locked quvyta-code'

New-Case 'build-tools-checked-for-msvc-host'
Remove-Item -Recurse (Join-Path $script:Root 'pf86')
Write-Stub 'rustc.exe' "printf 'rustc 1.95.0\nhost: x86_64-pc-windows-msvc\n'"
Invoke-Case @('-Yes', 'code')
Assert-Status 1
Assert-Output 'Visual Studio C++ Build Tools'

# --- PATH

New-Case 'path-added-once'
Invoke-Case @('-Yes', 'code')
Assert-Status 0
Assert-Output 'This folder would be added to the end of your user PATH setting:'
Assert-Output "  $(& $bin)"
Assert-Equal $script:UserPath ('C:\Windows\System32;%USERPROFILE%\AppData\Local\Microsoft\WindowsApps;' + (& $bin)) 'the user PATH'
if (-not (Test-QuvytaPathHas $env:PATH ([IO.Path]::PathSeparator) (& $bin))) { Fail "this session's PATH lacks cargo's folder" }
Assert-Output 'Open a new terminal'
# A new terminal: this session's PATH as it was, the stored one as it is now.
$env:PATH = $script:Stubs + [IO.Path]::PathSeparator + $script:BasePath
$script:Out.Clear()
Invoke-Case @('-Yes', 'code')
Assert-Output 'Your PATH setting already has it'
Assert-Equal $script:PathWrites 1 'the number of PATH writes'

New-Case 'path-already-in-user-path-other-spelling'
$script:UserPath = 'C:\Windows\System32;' + (& $bin).ToUpperInvariant() + '\'
Invoke-Case @('-Yes', 'code')
Assert-Status 0
Assert-Output 'Your PATH setting already has it; a new terminal will pick it up.'
Assert-Equal $script:PathWrites 0 'the number of PATH writes'

New-Case 'path-already-through-a-variable'
$env:QUVYTA_TEST_CARGO = $env:CARGO_HOME
$script:UserPath = '%QUVYTA_TEST_CARGO%/bin;C:\Windows'
Invoke-Case @('-Yes', 'code')
Assert-Output 'Your PATH setting already has it'
Assert-Equal $script:PathWrites 0 'the number of PATH writes'

New-Case 'path-already-in-machine-path'
$script:MachinePath = 'C:\Windows;' + (& $bin)
Invoke-Case @('-Yes', 'code')
Assert-Output 'Your PATH setting already has it'
Assert-Equal $script:PathWrites 0 'the number of PATH writes'

New-Case 'path-already-in-session'
$env:PATH = (& $bin) + [IO.Path]::PathSeparator + $env:PATH
Invoke-Case @('-Yes', 'code')
Assert-Status 0
Assert-Output 'Run a command above by its name to start it.'
Assert-NoOutput 'not on your PATH yet'
Assert-Equal $script:PathWrites 0 'the number of PATH writes'

New-Case 'path-user-path-empty'
$script:UserPath = ''
Invoke-Case @('-Yes', 'code')
Assert-Equal $script:UserPath (& $bin) 'the user PATH'

New-Case 'path-user-path-trailing-separator'
$script:UserPath = 'C:\Tools;'
Invoke-Case @('-Yes', 'code')
Assert-Equal $script:UserPath ('C:\Tools;' + (& $bin)) 'the user PATH'

New-Case 'path-user-path-not-text'
$script:UserPath = $null
Invoke-Case @('-Yes', 'code')
Assert-Status 0
Assert-Output 'is not stored as text, so it is left alone'
Assert-Equal $script:PathWrites 0 'the number of PATH writes'

New-Case 'path-write-fails'
function Write-QuvytaUserPath { throw 'Access is denied.' }
Invoke-Case @('-Yes', 'code')
Assert-Status 0
Assert-Output 'Could not change your PATH setting: Access is denied. Add the folder yourself.'
Assert-Output 'start them by their full path'
function Write-QuvytaUserPath {
    param([string]$Value)
    $script:UserPath = $Value
    $script:PathWrites++
}

# --- Failures

New-Case 'one-crate-fails-others-go-on'
$env:FAKE_FAIL = 'quvyta-code'
Invoke-Case @('-Yes', 'code', 'focus')
Assert-Status 1
Assert-Logged 'cargo install --locked quvyta-code'
Assert-Logged 'cargo install --locked quvyta-focus'
Assert-Output 'Could not install: quvyta-code. The messages above say why.'
Assert-Output 'Installed:'
Assert-Output '  qfocus   tracks what you focus on'

New-Case 'every-crate-fails'
$env:FAKE_FAIL = 'quvyta-code'
Invoke-Case @('-Yes', 'code')
Assert-Status 1
Assert-NoOutput 'Installed:'
Assert-Equal $script:PathWrites 0 'the number of PATH writes'

# --- How the real script starts and ends, in a fresh PowerShell each
#
# On Linux the script stops at its Windows check, which is enough to see that the last line runs
# the installer, which exit code it leaves and that the caller's session is left as it was.

function Invoke-Pwsh {
    param([string]$Command)
    $output = & pwsh -NoProfile -NonInteractive -Command $Command 2>&1 | Out-String
    return [pscustomobject]@{ Output = $output; Code = $LASTEXITCODE }
}

New-Case 'real-irm-iex'
$run = Invoke-Pwsh @'
$ErrorActionPreference = 'Continue'
Get-Content -Raw /src/install.ps1 | Invoke-Expression
"code=$LASTEXITCODE"
"eap=$ErrorActionPreference"
"strict-off=$($null -eq $notDefinedAnywhere)"
'@
foreach ($want in @('This installer is for Windows', 'install.sh | sh', 'code=1', 'eap=Continue', 'strict-off=True')) {
    if (-not $run.Output.Contains($want)) { Fail "iex run lacks '$want': $($run.Output)" }
}

New-Case 'real-scriptblock-with-arguments'
$run = Invoke-Pwsh @'
& ([scriptblock]::Create((Get-Content -Raw /src/install.ps1))) code -Yes --help
"code=$LASTEXITCODE"
& ([scriptblock]::Create((Get-Content -Raw /src/install.ps1))) code bogus
"code=$LASTEXITCODE"
'@
foreach ($want in @('Usage:', 'code=0', 'Unknown name: bogus', 'code=2')) {
    if (-not $run.Output.Contains($want)) { Fail "scriptblock run lacks '$want': $($run.Output)" }
}

New-Case 'real-file-exit-code'
$null = & pwsh -NoProfile -NonInteractive -File $script:Source --force 2>&1
Assert-Equal $LASTEXITCODE 2 'the exit code of a file run with an unknown option'
$null = & pwsh -NoProfile -NonInteractive -File $script:Source -Help 2>&1
Assert-Equal $LASTEXITCODE 0 'the exit code of a file run with -Help'

New-Case 'real-dot-source-runs-nothing'
$run = Invoke-Pwsh ". /src/install.ps1; 'defined=' + [bool](Get-Command Invoke-QuvytaInstaller)"
if ($run.Output.Trim() -ne 'defined=True') { Fail "dot-sourcing printed: $($run.Output)" }

New-Case 'real-cut-short-download-runs-nothing'
$lines = Get-Content -LiteralPath $script:Source
$half = ($lines[0..([int]($lines.Count / 2))] -join "`n")
$last = ($lines[0..($lines.Count - 2)] -join "`n") + "`n" + $lines[-1].Substring(0, 30)
foreach ($text in @($half, $last)) {
    $file = Join-Path $script:Root ('part-' + [guid]::NewGuid().ToString('N') + '.ps1')
    Set-Content -LiteralPath $file -Value $text
    $run = Invoke-Pwsh "Get-Content -Raw '$file' | Invoke-Expression"
    if ($run.Output.Contains('This installer is for Windows') -or $run.Output.Contains('Quvyta family')) {
        Fail "a cut-short script ran: $($run.Output)"
    }
}

New-Case 'real-console-check'
$run = Invoke-Pwsh ". /src/install.ps1; 'console=' + (Test-QuvytaConsole)"
if ($run.Output.Trim() -ne 'console=False') { Fail "a -NonInteractive session counts as a console: $($run.Output)" }

# --- Lint, when the analyzer module is at hand

if (Get-Module -ListAvailable -Name PSScriptAnalyzer) {
    New-Case 'script-analyzer'
    $found = @(Invoke-ScriptAnalyzer -Path $script:Source)
    foreach ($finding in $found) { Fail "$($finding.RuleName) at line $($finding.Line): $($finding.Message)" }
    # Windows PowerShell 5.1 on Windows 10 and Server 2019, from the profiles the analyzer ships.
    $windows51 = 'win-48_x64_10.0.17763.0_5.1.17763.316_x64_4.0.30319.42000_framework'
    $compatibility = @(Invoke-ScriptAnalyzer -Path $script:Source -Settings @{
            Rules = @{
                PSUseCompatibleSyntax = @{ Enable = $true; TargetVersions = @('5.1', '7.0') }
                PSUseCompatibleCommands = @{ Enable = $true; TargetProfiles = @($windows51) }
                PSUseCompatibleTypes = @{ Enable = $true; TargetProfiles = @($windows51) }
            }
            IncludeRules = @('PSUseCompatibleSyntax', 'PSUseCompatibleCommands', 'PSUseCompatibleTypes')
        })
    foreach ($finding in $compatibility) { Fail "$($finding.RuleName) at line $($finding.Line): $($finding.Message)" }
} else {
    Write-Host 'skipped PSScriptAnalyzer: the module is not installed in this container'
}

Remove-Item -Recurse -Force -LiteralPath $script:Work
if ($script:Failures -gt 0) {
    Write-Host "$($script:Failures) failed"
    exit 1
}
Write-Host 'install.ps1: all cases passed'
