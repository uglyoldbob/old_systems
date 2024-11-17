library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;
library max10_hdmi_output;

entity max10_nes_system is
	Port (
		clock: in std_logic;
		hdmi_d_p: out std_logic_vector(2 downto 0);
		hdmi_d_n: out std_logic_vector(2 downto 0);
		hdmi_ck_p: out std_logic;
		hdmi_ck_n: out std_logic;
		hdmi_cec: inout std_logic;
		hdmi_i2c_scl: inout std_logic;
		hdmi_i2c_sda: inout std_logic;
		hdmi_hpd: inout std_logic;
		sd_d: inout std_logic_vector(3 downto 0);
		sd_ck: out std_logic;
		sd_cmd: out std_logic);
end max10_nes_system;

architecture Behavioral of max10_nes_system is
	signal ppu_r: std_logic_vector(7 downto 0);
	signal ppu_g: std_logic_vector(7 downto 0);
	signal ppu_b: std_logic_vector(7 downto 0);
	
	signal cpu_clock: std_logic := '0';
	signal cpu_reset: std_logic := '0';
	signal cpu_address: std_logic_vector(15 downto 0);
	signal cpu_dout: std_logic_vector(7 downto 0);
	signal cpu_din: std_logic_vector(7 downto 0);
	signal cpu_oe: std_logic_vector(1 downto 0);
	
	signal write_signal: std_logic;
	signal write_address: std_logic_vector(19 downto 0);
	signal write_value: std_logic_vector(7 downto 0);
	signal write_trigger: std_logic;
	signal write_rw: std_logic;
	signal write_cs: std_logic_vector(1 downto 0);
	
	signal pll_lock: std_logic;
	
	signal hdmi_pixel_clock: std_logic;
	signal tmds_clock: std_logic; --5x the pixel clock
	
	signal tmds10_0: std_logic_vector(9 downto 0);
	signal tmds10_1: std_logic_vector(9 downto 0);
	signal tmds10_2: std_logic_vector(9 downto 0);
	
	signal tmds_0_post: std_logic_vector(0 downto 0);
	signal tmds_1_post: std_logic_vector(0 downto 0);
	signal tmds_2_post: std_logic_vector(0 downto 0);
	signal tmds_clk_post: std_logic_vector(0 downto 0);
	signal tmds_0b_post: std_logic_vector(0 downto 0);
	signal tmds_1b_post: std_logic_vector(0 downto 0);
	signal tmds_2b_post: std_logic_vector(0 downto 0);
	signal tmds_clkb_post: std_logic_vector(0 downto 0);
	
	signal hdmi_tmds_0: std_logic_vector(1 downto 0);
	
	signal random_data: std_logic_vector(31 downto 0);
begin
	nes: entity work.nes port map (
		ppu_r => ppu_r,
		ppu_g => ppu_g,
		ppu_b => ppu_b,
		write_signal => write_signal,
		write_address => write_address,
		write_value => write_value,
		write_trigger => write_trigger,
		write_rw => write_rw,
		write_cs => write_cs,
		reset => cpu_reset,
		cpu_oe => cpu_oe,
		cpu_memory_address => cpu_address,
		clock => cpu_clock
		);
	
	hdmi_pll_inst : work.hdmi_pll PORT MAP (
		inclk0 => clock,
		c0 => tmds_clock,
		locked => pll_lock);

	hdmi_converter: entity work.hdmi generic map(
		hsync_polarity => '1',
		vsync_polarity => '1',
		h => 1280,
		v => 720,
		hblank_width => 370,
		hsync_porch => 220,
		hsync_width => 40,
		vblank_width => 30,
		vsync_porch => 20,
		vsync_width => 5) port map(
		reset => not pll_lock,
		pixel_clock => hdmi_pixel_clock,
		tmds_clock => tmds_clock,
		tmds_0 => tmds10_0,
		tmds_1 => tmds10_1,
		tmds_2 => tmds10_2,
		cec => hdmi_cec,
		i2c_scl => hdmi_i2c_scl,
		i2c_sda => hdmi_i2c_sda,
		hpd => hdmi_hpd,
		r => "00101010",
		g => "01010101",
		b => "01110011");

	tmds_reducer: entity work.tmds_multiplexer port map(
		clock => tmds_clock,
		pixel_clock => hdmi_pixel_clock,
		reset => '0',
		din => tmds10_0,
		dout => hdmi_tmds_0);
		
	hdmi_putter: entity max10_hdmi_output.max10_hdmi_output port map(
		outclock => tmds_clock,
		din => hdmi_tmds_0,
		pad_out => tmds_0_post,
		pad_out_b => tmds_0b_post);
	hdmi_d_p <= tmds_0_post & tmds_1_post & tmds_2_post;
	hdmi_d_n <= tmds_0b_post & tmds_1b_post & tmds_2b_post;
	hdmi_ck_p <= tmds_clk_post(0);
	hdmi_ck_n <= tmds_clkb_post(0);

	random: entity work.lfsr32 port map(
		clock => hdmi_pixel_clock,
		dout => random_data);
end Behavioral;