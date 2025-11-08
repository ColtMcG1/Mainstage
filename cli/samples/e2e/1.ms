stage app () {
    name = "Demo App"
}

project core_lib {
    root = "../core"
    description = "Core library"
    enabled = true
    version = 1

    dependencies = [app]
}

workspace demo_ws {
    members = [core_lib]

    default_project = members[0]
}