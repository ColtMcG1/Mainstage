include "./2.ms"

project mainstage_core {
    root = "../core"
}

project mainstage_cli {
    root = "../cli"
}

workspace mainstage {
    members = [mainstage_core, mainstage_cli]
}