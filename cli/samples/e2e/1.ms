workspace optimizer_demo {
    projects = [opt_project];
    for project in projects { say("Workspace start"); }
}

project opt_project {
    // constant folding: (1+2)*(3+4) -> 21
    say((1 + 2) * (3 + 4));

    // CSE: use identical pure expressions directly so CSE can deduplicate
    say((2 + 3) * (4 + 5));
    say((2 + 3) * (4 + 5));

    // SCCP-friendly: different expressions that evaluate to same constant
    say(10 - 5);
    say(2 + 3);

    // copy propagation + const propagation through locals
    tmp = "hello";
    a = tmp;
    b = tmp;
    say(a);
    say(b.length()); // should fold to 5

    // repeated pure computation (CSE opportunity)
    say(6 * 7);
    say(6 * 7);

    // loop/side-effect example (ensures optimizer is conservative with mutable ops)
    arr = ["x", "y", "z"];
    n = arr.length();   // folded -> 3
    while(n) {
        say(arr[n]);
        --n;
    }

    // dead code elimination: unused result should be removed eventually
    unused = (10 * 2) + (30 / 3) + (4 - 1);
}

stage echo_stage (val) {
    return val;
}