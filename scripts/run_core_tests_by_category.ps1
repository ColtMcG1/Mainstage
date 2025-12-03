# Run core integration tests by category
# Usage examples:
#  .\run_core_tests_by_category.ps1 -Category lowering
#  .\run_core_tests_by_category.ps1 -Category opt
#  .\run_core_tests_by_category.ps1 -Category all

param(
    [string]$Category = "all"
)

# Map categories to integration test names (file basenames without .rs)
$map = @{
    "lowering" = @(
        "lowering_forin",
        "lowering_calls",
        "lowering_loop",
        "lowering_loop_exhaustive",
        "ir_lowering"
    )
    "ir" = @(
        "ir_patch"
    )
    "opt" = @(
        "opt_plugin_preserve",
        "opt_const_canon_extern_vis"
    )
    "emit" = @(
        "emit_failing_bytecode",
        "emit_control_and_bytecode"
    )
    "util" = @(
        "util_read_glob_workdir",
        "util_template_test"
    )
    "all" = @("__all__")
}

# Run tests
Set-Location -Path "$(Split-Path -Parent $PSScriptRoot)\core"

if ($Category -eq 'all') {
    Write-Host "Running entire core test suite..."
    cargo test
    exit $LASTEXITCODE
}

if (-not $map.ContainsKey($Category)) {
    Write-Host "Unknown category: $Category"
    Write-Host "Available categories: $($map.Keys -join ', ')"
    exit 2
}

$tests = $map[$Category]
if ($tests -contains '__all__') {
    Write-Host "Running entire core test suite..."
    cargo test
    exit $LASTEXITCODE
}

foreach ($t in $tests) {
    Write-Host "Running test: $t"
    cargo test --test $t
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Write-Host "All selected tests passed."