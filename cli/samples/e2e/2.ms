workspace demo_ws {
    projects = [test_pj];

    for project in projects
    {
        say(6);
    }
}

project test_pj {
    test = "What should I say?";
    out = demo_stage(test);
    say(out.length());
}

stage demo_stage (var) // No arguments
{
    say("I should say - " + var);
    return var;
}