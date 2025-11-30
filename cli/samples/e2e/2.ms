[entrypoint]
workspace demo_ws {
    projects = [test_pj];

    for p in projects
    {
        say("Processing project:");
        say(p.sources);
    }
}

project test_pj {
    sources = ["./samples/e2e/*.ms"];
}

stage load_stage(var)
{
    say(var);
    in = read(var);
    say(in);
    return in;
}

stage process_project_stage(prj)
{
    if prj.sources == null {
        say("No sources found.");
        return;
    }
    var = prj.sources[0];
    say(var);
    in = load_stage(var);
    say(in);
}