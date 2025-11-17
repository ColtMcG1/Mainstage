workspace demo_ws {
    projects = [test_pj];  // list of project references; Should call project init for all projects in list since this is their first reference.

    for project in projects {
        say(project); // Should print something like: Ref(ProjectId(1))
    }

}

project test_pj {
    demo_stage();           // Should call stage init for demo_stage below
    test1 = demo_stage.test; // Should get the value from the initalized stage below
}

stage demo_stage () // No arguments
{
    test = ask("Test number > "); // Should prompt user for a number and store it in 'test'
}