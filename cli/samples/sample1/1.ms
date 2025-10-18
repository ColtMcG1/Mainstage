include "./2.ms"

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

workspace mainstage {
    members = [mainstage_core, mainstage_cli]
}