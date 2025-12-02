workspace fmt_ws {
	say(fmt("Limiting precision to 1 place, {:.1}", 12.34));
	say(fmt("Example combined: {:>10.1}", 12.34));
	say(fmt("Original-style example: {:4.1}", 12.34));
	say(fmt("Width+prec example: {:8.1}", 12.34));
}
