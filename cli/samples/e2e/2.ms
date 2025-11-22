workspace demo_ws {
    projects = [test_pj];
}

project test_pj {
    test = "What should I say?";
    out = demo_stage(test);
    say(out.length());
}

stage demo_stage (var)
{
    say("I should say - " + var);
    return var;
}