stage sample_app () {
    name = "MainStage CLI Sample App"
    description = "A sample application demonstrating MainStage CLI features."
}

include "./2.ms"

// A project can have various builtin attributes applied to it. Such as "entrypoint" to specify the main entry point file for the project. 
[entrypoint, test]
project mainstage_core {
    root = "../core"
}

project mainstage_cli {
    root = "../cli"
    test_bool = true
    test_string = "hello"
    test_number = 42
    test_array = [1, 2, 3]
    test_shell = zsh"echo 'This is a shell command'"
}

// If no entry point is specified, the workspace will use the first project listed.
workspace mainstage {
    members = [mainstage_core, mainstage_cli]
}