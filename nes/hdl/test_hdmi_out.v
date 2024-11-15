module test_hdmi_out (
	input clk_pixel,
	input [2:0] tmds,
	output       tmds_clk_n,
	output       tmds_clk_p,
	output [2:0] tmds_d_n,
	output [2:0] tmds_d_p
);

	ELVDS_OBUF tmds_bufds [3:0] (
		.I({clk_pixel, tmds}),
		.O({tmds_clk_p, tmds_d_p}),
		.OB({tmds_clk_n, tmds_d_n})
	);

endmodule