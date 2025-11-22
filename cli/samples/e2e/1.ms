workspace optimizer_demo {
    projects = [opt_project];
    for project in projects { say("Workspace start"); }
}

project opt_project {
    // Constant expression folded: (1 + 2) * (3 + 4) => 21
    say((1 + 2) * (3 + 4));

    // String concatenation folded: "foo" + "bar" => "foobar"
    msg = "foo" + "bar";
    say(msg);

    // Length folded: constant string length => 5
    word = "hello";
    say(word.length());

    // Entire call chain returns constant -> length also constant
    out = echo_stage("abc");
    say(out.length());

    // Unused complex expression temps eliminated (DCE):
    unused = (10 * 2) + (30 / 3) + (4 - 1); // never referenced
}

stage echo_stage (val) {
    // Return value directly; length of "abc" folded at call site
    return val;
}