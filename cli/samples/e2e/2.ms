workspace demo_ws {
    projects = [test_pj];

    for project in projects
    {
        say(project.out);
    }
}

project test_pj {
    test = ask("What should I say?");
    out = demo_stage(test);
    say(out.length());
}

stage demo_stage (var) // No arguments
{
    say("I should say - " + var);
    return var;
}