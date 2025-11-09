task compute() {
    return 42
}

stage app () {
    name = "Demo App"
    result = compute()
    say(result)
}

project core_lib {
    root = "../core"
    description = "Core library"
    enabled = true
    version = 1
}

workspace demo_ws {
    members = [core_lib]

    default_project = members[0]
}