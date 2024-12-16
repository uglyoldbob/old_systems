library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity nes_tang_nano_20k_tb is
end nes_tang_nano_20k_tb;

architecture Behavioral of nes_tang_nano_20k_tb is
	constant rambits: integer := 3;

	signal sdram_clk: std_logic;
	signal sdram_cke: std_logic;
	signal sdram_cs_n: std_logic;
	signal sdram_cas_n: std_logic;
	signal sdram_ras_n: std_logic;
	signal sdram_wen_n: std_logic;
	signal sdram_dqm: std_logic_vector(3 downto 0);
	signal sdram_addr: std_logic_vector(10 downto 0);
	signal sdram_ba: std_logic_vector(1 downto 0);
	signal sdram_dq: std_logic_vector(31 downto 0);

	signal nes_reset: std_logic := '0';
	signal hdmi_pixel_clock: std_logic := '0';
	signal clk27: std_logic := '0';
begin

	hdmi_pixel_clock <= not hdmi_pixel_clock after 6734 ps;
	clk27 <= not clk27 after 18519 ps;

	process (all)
	begin
		nes_reset <= '1';
		nes_reset <= '0' after 100 ns;
	end process;

	uut: entity work.nes_tang_nano_20k port map(
        reset => nes_reset,
        clock => clk27,
		buttons => "00",
		O_sdram_clk => sdram_clk,
		O_sdram_cke => sdram_cke,
		O_sdram_cs_n => sdram_cs_n,
		O_sdram_cas_n => sdram_cas_n,
		O_sdram_ras_n => sdram_ras_n,
		O_sdram_wen_n => sdram_wen_n,
		O_sdram_dqm => sdram_dqm,
		O_sdram_addr => sdram_addr,
		O_sdram_ba => sdram_ba,
		IO_sdram_dq => sdram_dq);
end Behavioral;

