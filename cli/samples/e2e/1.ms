task compute() {
    return 42
}

stage app () {
    result = compute()
    say(compute())
}

project core_lib {

}

workspace demo_ws {
    members = [core_lib]

    default_project = members[0]

    app()
}