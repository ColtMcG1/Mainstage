workspace optimizer_demo {
    projects = [opt_project];
    for project in projects { say("Workspace start"); }
}

project opt_project {
    // constant folding: (1+2)*(3+4) -> 21
    say((1 + 2) * (3 + 4));

    // string concatenation folded -> "foobar"
    msg = "foo" + "bar";
    say(msg);

    // copy propagation: tmp is copy of a -> use replaced with constant/local where possible
    a = "copyme";
    tmp = a;
    say(tmp);

    // constant propagation through locals
    len_local = "hello".length();
    stored_len = len_local;
    say(stored_len); // should fold to 5

    // fold across calls where possible (echo_stage returns its arg)
    out = echo_stage("abc");
    say(out.length()); // folded -> 3

    // loop with constant array length: length folds but index remains mutable (no incorrect folding)
    arr = ["x", "y", "z"];
    n = arr.length();   // folded -> 3
    while(n) {
        say(arr[n]);
        --n;
    }

    // dead code elimination: unused result should be removed
    unused = (10 * 2) + (30 / 3) + (4 - 1);
}

stage echo_stage (val) {
    return val;
}