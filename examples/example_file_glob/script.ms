[entrypoint]
workspace demo_ws {
    projects = [test_pj];

    for p in projects
    {
        process_project_stage(p);
    }
}

project test_pj {
    sources = ["./data/*.txt"];
}

stage load_sources_stage(var)
{
    in = read(var);
    return in;
}

stage process_project_stage(prj)
{
    if prj.sources == null {
        say("No sources found.");
        return;
    }
    sources = load_sources_stage(prj.sources[0]);
    if sources == null {
        say("No sources loaded.");
        return;
    } else {
        say("Loaded sources: " + sources.length);
    }
    for src in sources {
        say(src);
        say("----");
    }
}