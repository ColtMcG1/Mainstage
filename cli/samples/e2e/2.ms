[entrypoint]
workspace demo_ws {
    projects = [test_pj];

    for p in projects
    {
        process_project_stage(p);
    }
}

project test_pj {
    sources = ["./samples/e2e/*.ms"];
}

stage load_stage(var)
{
    return read(var);
}

stage process_project_stage(prj)
{
    if prj.sources == null {
        say("No sources found.");
        return;
    }
    in = load_stage(prj.sources[0]);
    say(in);
}