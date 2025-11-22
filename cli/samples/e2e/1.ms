workspace demo_ws {
    projects = [test_pj];

    for project in projects
    {
        demo_stage("Hello");
    }
}

project test_pj {
    test = "Hello world";
}

stage demo_stage (var) // No arguments
{
    say(var);
}