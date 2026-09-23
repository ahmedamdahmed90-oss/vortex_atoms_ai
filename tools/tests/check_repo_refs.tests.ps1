#!/usr/bin/env pwsh
# Copyright (c) 2026 Ahmad Mansour. All rights reserved.
# Pester tests for tools/check_repo_refs.ps1 (REPO-01 Section 2).

#Requires -Modules @{ ModuleName = 'Pester'; ModuleVersion = '5.0.0' }

BeforeAll {
    $script:ScriptPath = (Resolve-Path (Join-Path (Join-Path $PSScriptRoot '..') 'check_repo_refs.ps1')).Path
    $script:RepoRoot = (Resolve-Path (Join-Path (Join-Path (Join-Path $PSScriptRoot '..') '..') '..')).Path
    $script:CanonicalRepo = 'vortex_atoms_ai'
    $script:BadgePattern = 'github\.com/ahmedamdahmed90-oss/vortex_atoms_ai'
    $script:ContactEmail = 'vortexatoms@gmail.com'

    function New-FixtureRepo {
        param(
            [string]$RepoUrl = 'https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai',
            [string]$HeadSha,
            [string]$ReadmeContent = "# Vortex`n`n[badge](https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai/actions)",
            [string]$IndexContent = '- [Repo](https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai)',
            [string]$OutreachContent = "Contact: vortexatoms@gmail.com",
            [switch]$OmitIndex,
            [switch]$OmitOutreach
        )

        $dir = Join-Path ([System.IO.Path]::GetTempPath()) ("pester_repo_refs_" + [guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        Push-Location $dir
        try {
            git init -q | Out-Null
            if ($LASTEXITCODE -ne 0 -or -not (Test-Path (Join-Path '.git' 'HEAD'))) {
                throw "git init failed in $dir"
            }
            git config user.email 'pester@example.com'
            git config user.name 'Pester'
            git config commit.gpgsign false

            Set-Content -Path 'README.md' -Value $ReadmeContent -Encoding UTF8
            New-Item -ItemType Directory -Path 'docs' -Force | Out-Null
            if (-not $OmitIndex) {
                Set-Content -Path (Join-Path 'docs' 'index.md') -Value $IndexContent -Encoding UTF8
            }
            if (-not $OmitOutreach) {
                Set-Content -Path (Join-Path 'docs' 'OUTREACH_KIT.md') -Value $OutreachContent -Encoding UTF8
            }

            git add -A | Out-Null
            git commit -q -m 'fixture' | Out-Null
            if ($LASTEXITCODE -ne 0) {
                throw "git commit failed in $dir"
            }

            $actualSha = (git rev-parse HEAD 2>$null | Out-String).Trim()
            if ([string]::IsNullOrWhiteSpace($actualSha)) {
                throw "git rev-parse HEAD returned empty in $dir"
            }
            if (-not $PSBoundParameters.ContainsKey('HeadSha') -or [string]::IsNullOrWhiteSpace($HeadSha)) {
                $HeadSha = $actualSha
            }

            git remote add origin $RepoUrl 2>$null | Out-Null

            $facts = [ordered]@{
                repository = [ordered]@{
                    repo_url = @{ value = $RepoUrl }
                    repo_sha = @{ value = $HeadSha }
                    repo_name = @{ value = $script:CanonicalRepo }
                }
            }
            $factsPath = Join-Path $dir 'facts.json'
            ($facts | ConvertTo-Json -Depth 6) | Set-Content -Path $factsPath -Encoding UTF8

            [pscustomobject]@{
                Dir        = $dir
                FactsPath  = $factsPath
                ActualSha  = $actualSha
                RequestedSha = $HeadSha
            }
        }
        finally {
            Pop-Location
        }
    }

    function Invoke-CheckRepoRefs {
        param(
            [Parameter(Mandatory)]
            [string]$FixtureDir,
            [Parameter(Mandatory)]
            [string]$FactsPath
        )
        $prev = Get-Location
        Push-Location $FixtureDir
        try {
            $output = & $script:ScriptPath -FactsPath $FactsPath *>&1 | Out-String
            [pscustomobject]@{
                ExitCode = $LASTEXITCODE
                Output   = $output
            }
        }
        catch {
            [pscustomobject]@{
                ExitCode = 1
                Output   = $_.Exception.Message
            }
        }
        finally {
            Pop-Location
            Set-Location $prev
        }
    }
}

AfterAll {
    # Only clean fixtures still present; scoped filter avoids clobbering parallel runs.
    Get-ChildItem -Path ([System.IO.Path]::GetTempPath()) -Directory -Filter 'pester_repo_refs_*' -ErrorAction SilentlyContinue |
        Where-Object { $_.LastWriteTime -lt (Get-Date).AddMinutes(-5) } |
        Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
}

Describe 'check_repo_refs.ps1' {
    Context 'happy path (canonical remote, matching SHA, good docs)' {
        BeforeAll {
            $script:Fix = New-FixtureRepo
            $script:Result = Invoke-CheckRepoRefs -FixtureDir $script:Fix.Dir -FactsPath $script:Fix.FactsPath
        }

        AfterAll {
            if ($script:Fix -and (Test-Path $script:Fix.Dir)) {
                Remove-Item -Recurse -Force -ErrorAction SilentlyContinue -Path $script:Fix.Dir
            }
        }

        It 'exits 0' {
            $script:Result.ExitCode | Should -Be 0
        }

        It 'reports git remote OK' {
            $script:Result.Output | Should -Match 'Git remote OK'
        }

        It 'reports HEAD SHA match' {
            $script:Result.Output | Should -Match 'HEAD SHA matches'
        }

        It 'reports README badges OK' {
            $script:Result.Output | Should -Match 'README badges reference this repo'
        }

        It 'reports docs/index.md OK' {
            $script:Result.Output | Should -Match 'docs/index\.md references this repo'
        }

        It 'reports OUTREACH_KIT contact OK' {
            $script:Result.Output | Should -Match 'OUTREACH_KIT\.md contact email present'
        }

        It 'prints all-clear banner' {
            $script:Result.Output | Should -Match 'ALL REPO REFS VERIFIED'
        }
    }

    Context 'git remote mismatch fails' {
        BeforeAll {
            $script:Fix = New-FixtureRepo -RepoUrl 'https://github.com/someone/else_project'
            $script:Result = Invoke-CheckRepoRefs -FixtureDir $script:Fix.Dir -FactsPath $script:Fix.FactsPath
        }

        AfterAll {
            if ($script:Fix -and (Test-Path $script:Fix.Dir)) {
                Remove-Item -Recurse -Force -ErrorAction SilentlyContinue -Path $script:Fix.Dir
            }
        }

        It 'exits 1' {
            $script:Result.ExitCode | Should -Be 1
        }

        It 'reports remote mismatch' {
            $script:Result.Output | Should -Match 'Git remote does not match'
        }

        It 'prints failed-check banner' {
            $script:Result.Output | Should -Match 'CHECK\(S\) FAILED'
        }
    }

    Context 'HEAD SHA mismatch with unknown SHA fails (non-shallow)' {
        BeforeAll {
            $script:Fix = New-FixtureRepo -HeadSha '0000000000000000000000000000000000000000'
            $script:Result = Invoke-CheckRepoRefs -FixtureDir $script:Fix.Dir -FactsPath $script:Fix.FactsPath
        }

        AfterAll {
            if ($script:Fix -and (Test-Path $script:Fix.Dir)) {
                Remove-Item -Recurse -Force -ErrorAction SilentlyContinue -Path $script:Fix.Dir
            }
        }

        It 'exits 1' {
            $script:Result.ExitCode | Should -Be 1
        }

        It 'reports SHA not found in repo' {
            $script:Result.Output | Should -Match 'HEAD SHA mismatch AND facts\.json SHA not found'
        }
    }

    Context 'HEAD SHA differs but is a valid ancestor commit passes with warning' {
        BeforeAll {
            $parentDir = Join-Path ([System.IO.Path]::GetTempPath()) ("pester_repo_refs_" + [guid]::NewGuid().ToString('N'))
            New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
            Push-Location $parentDir
            try {
                git init -q 2>$null | Out-Null
                git config user.email 'pester@example.com'
                git config user.name 'Pester'
                git config commit.gpgsign false
                Set-Content -Path 'README.md' -Value "[badge](https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai)" -Encoding UTF8
                New-Item -ItemType Directory -Path 'docs' -Force | Out-Null
                Set-Content -Path (Join-Path 'docs' 'index.md') -Value 'https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai' -Encoding UTF8
                Set-Content -Path (Join-Path 'docs' 'OUTREACH_KIT.md') -Value 'vortexatoms@gmail.com' -Encoding UTF8
                git add -A 2>$null | Out-Null
                git commit -q -m 'first' 2>$null | Out-Null
                $ancestorSha = (git rev-parse HEAD).Trim()
                Set-Content -Path 'README.md' -Value "[badge2](https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai)" -Encoding UTF8
                git add -A 2>$null | Out-Null
                git commit -q -m 'second' 2>$null | Out-Null
                git remote add origin 'https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai' 2>$null | Out-Null

                $facts = [ordered]@{
                    repository = [ordered]@{
                        repo_url  = @{ value = 'https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai' }
                        repo_sha  = @{ value = $ancestorSha }
                        repo_name = @{ value = 'vortex_atoms_ai' }
                    }
                }
                $factsPath = Join-Path $parentDir 'facts.json'
                ($facts | ConvertTo-Json -Depth 6) | Set-Content -Path $factsPath -Encoding UTF8
                $script:FixDir = $parentDir
                $script:FixFacts = $factsPath
            }
            finally {
                Pop-Location
            }
            $script:Result = Invoke-CheckRepoRefs -FixtureDir $script:FixDir -FactsPath $script:FixFacts
        }

        AfterAll {
            if ($script:FixDir -and (Test-Path $script:FixDir)) {
                Remove-Item -Recurse -Force -ErrorAction SilentlyContinue -Path $script:FixDir
            }
        }

        It 'exits 0' {
            $script:Result.ExitCode | Should -Be 0
        }

        It 'warns about SHA drift' {
            $script:Result.Output | Should -Match 'HEAD SHA differs from facts\.json'
        }
    }

    Context 'README missing canonical badge fails' {
        BeforeAll {
            $script:Fix = New-FixtureRepo -ReadmeContent "# No badges here`nJust text"
            $script:Result = Invoke-CheckRepoRefs -FixtureDir $script:Fix.Dir -FactsPath $script:Fix.FactsPath
        }

        AfterAll {
            if ($script:Fix -and (Test-Path $script:Fix.Dir)) {
                Remove-Item -Recurse -Force -ErrorAction SilentlyContinue -Path $script:Fix.Dir
            }
        }

        It 'exits 1' {
            $script:Result.ExitCode | Should -Be 1
        }

        It 'reports README badge failure' {
            $script:Result.Output | Should -Match 'README badges do not reference this repo'
        }
    }

    Context 'docs/index.md present but missing repo link fails' {
        BeforeAll {
            $script:Fix = New-FixtureRepo -IndexContent '- [Elsewhere](https://github.com/example/other)'
            $script:Result = Invoke-CheckRepoRefs -FixtureDir $script:Fix.Dir -FactsPath $script:Fix.FactsPath
        }

        AfterAll {
            if ($script:Fix -and (Test-Path $script:Fix.Dir)) {
                Remove-Item -Recurse -Force -ErrorAction SilentlyContinue -Path $script:Fix.Dir
            }
        }

        It 'exits 1' {
            $script:Result.ExitCode | Should -Be 1
        }

        It 'reports index.md failure' {
            $script:Result.Output | Should -Match 'docs/index\.md does not reference this repo'
        }
    }

    Context 'docs/index.md missing is tolerated' {
        BeforeAll {
            $script:Fix = New-FixtureRepo -OmitIndex
            $script:Result = Invoke-CheckRepoRefs -FixtureDir $script:Fix.Dir -FactsPath $script:Fix.FactsPath
        }

        AfterAll {
            if ($script:Fix -and (Test-Path $script:Fix.Dir)) {
                Remove-Item -Recurse -Force -ErrorAction SilentlyContinue -Path $script:Fix.Dir
            }
        }

        It 'exits 0' {
            $script:Result.ExitCode | Should -Be 0
        }

        It 'still reports index OK (absent file is skipped)' {
            $script:Result.Output | Should -Match 'docs/index\.md references this repo'
        }
    }

    Context 'OUTREACH_KIT present but missing contact email fails' {
        BeforeAll {
            $script:Fix = New-FixtureRepo -OutreachContent 'No contact details'
            $script:Result = Invoke-CheckRepoRefs -FixtureDir $script:Fix.Dir -FactsPath $script:Fix.FactsPath
        }

        AfterAll {
            if ($script:Fix -and (Test-Path $script:Fix.Dir)) {
                Remove-Item -Recurse -Force -ErrorAction SilentlyContinue -Path $script:Fix.Dir
            }
        }

        It 'exits 1' {
            $script:Result.ExitCode | Should -Be 1
        }

        It 'reports missing contact email' {
            $script:Result.Output | Should -Match 'OUTREACH_KIT\.md missing contact email'
        }
    }

    Context 'OUTREACH_KIT missing is tolerated' {
        BeforeAll {
            $script:Fix = New-FixtureRepo -OmitOutreach
            $script:Result = Invoke-CheckRepoRefs -FixtureDir $script:Fix.Dir -FactsPath $script:Fix.FactsPath
        }

        AfterAll {
            if ($script:Fix -and (Test-Path $script:Fix.Dir)) {
                Remove-Item -Recurse -Force -ErrorAction SilentlyContinue -Path $script:Fix.Dir
            }
        }

        It 'exits 0' {
            $script:Result.ExitCode | Should -Be 0
        }

        It 'still reports outreach OK (absent file is skipped)' {
            $script:Result.Output | Should -Match 'OUTREACH_KIT\.md contact email present'
        }
    }

    Context 'missing facts.json fails hard' {
        BeforeAll {
            $dir = Join-Path ([System.IO.Path]::GetTempPath()) ("pester_repo_refs_" + [guid]::NewGuid().ToString('N'))
            New-Item -ItemType Directory -Path $dir -Force | Out-Null
            Push-Location $dir
            try {
                git init -q 2>$null | Out-Null
                Set-Content -Path 'README.md' -Value 'x' -Encoding UTF8
                git add -A 2>$null | Out-Null
                git commit -q -m 'x' 2>$null | Out-Null
                git remote add origin 'https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai' 2>$null | Out-Null
            }
            finally {
                Pop-Location
            }
            $script:MissingDir = $dir
            $script:MissingFacts = Join-Path $dir 'does_not_exist.json'
            $script:Result = Invoke-CheckRepoRefs -FixtureDir $dir -FactsPath $script:MissingFacts
        }

        AfterAll {
            if ($script:MissingDir -and (Test-Path $script:MissingDir)) {
                Remove-Item -Recurse -Force -ErrorAction SilentlyContinue -Path $script:MissingDir
            }
        }

        It 'does not exit 0' {
            $script:Result.ExitCode | Should -Not -Be 0
        }

        It 'reports missing facts file' {
            $script:Result.Output | Should -Match 'does not exist|Cannot find path|Could not find'
        }
    }
}
