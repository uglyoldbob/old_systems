module sim_finish (
    input clk,
    input trigger
);
    always @(posedge clk) begin
        if (trigger) begin
            $display("Simulation finished!");
            $finish;
        end
    end
endmodule
