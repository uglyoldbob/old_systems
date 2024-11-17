library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity nes_tang_nano_20k is
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
        sd_cmd: out std_logic;
        buttons: in std_logic_vector(1 downto 0);
        test: out std_logic;
        leds: out std_logic_vector(5 downto 0));
end nes_tang_nano_20k;

architecture Behavioral of nes_tang_nano_20k is
    signal random_data: std_logic_vector(31 downto 0);
    signal hdmi_pixel_clock: std_logic;

    signal pll_lock: std_logic;

    signal tmds_clock: std_logic;

    signal tmds10_0: std_logic_vector(9 downto 0);
	signal tmds10_1: std_logic_vector(9 downto 0);
	signal tmds10_2: std_logic_vector(9 downto 0);

	signal tmds_0: std_logic_vector(0 downto 0);
	signal tmds_1: std_logic_vector(0 downto 0);
	signal tmds_2: std_logic_vector(0 downto 0);

    signal tmds_0_post: std_logic_vector(0 downto 0);
    signal tmds_1_post: std_logic_vector(0 downto 0);
    signal tmds_2_post: std_logic_vector(0 downto 0);
    signal tmds_clk_post: std_logic_vector(0 downto 0);

    signal tmds_0_ddr: std_logic_vector(1 downto 0);
	signal tmds_1_ddr: std_logic_vector(1 downto 0);
	signal tmds_2_ddr: std_logic_vector(1 downto 0);

	signal tmds: std_logic_vector(2 downto 0);

    component tmds_pll
        port (
            clkout: out std_logic;
            lock: out std_logic;
            clkin: in std_logic
        );
    end component;

    component tiny_hdmi_pll
        port (
            clkout: out std_logic;
            clkin: in std_logic
        );
    end component;

    component slow_pll
        port (
            clkout: out std_logic;
            clkin: in std_logic
        );
    end component;

    component tmds_div
        port (
            clkout: out std_logic;
            hclkin: in std_logic;
            resetn: in std_logic
        );
    end component;

    component Gowin_DDR
        port (
            din: in std_logic_vector(9 downto 0);
            fclk: in std_logic;
            pclk: in std_logic;
            reset: in std_logic;
            q: out std_logic_vector(0 downto 0)
        );
    end component;

	component test_hdmi_out
		port (
			clk_pixel: in std_logic;
			tmds: in std_logic_vector(2 downto 0);
			tmds_clk_n: out std_logic;
			tmds_clk_p: out std_logic;
			tmds_d_n: out std_logic_vector(2 downto 0);
			tmds_d_p: out std_logic_vector(2 downto 0));
	end component;

begin
    leds(5 downto 2) <= "1010";
    leds(1) <= not pll_lock;
    leds(0) <= not hdmi_hpd;

	tmds <= tmds_0 & tmds_1 & tmds_2;

	tmds_buf: test_hdmi_out port map (
		clk_pixel => hdmi_pixel_clock,
		tmds => tmds,
		tmds_clk_p => hdmi_ck_p,
		tmds_clk_n => hdmi_ck_n,
		tmds_d_p => hdmi_d_p,
		tmds_d_n => hdmi_d_n);

    hdmi_clock_ser: Gowin_DDR
        port map (
            din => "1111100000",
            fclk => tmds_clock,
            pclk => hdmi_pixel_clock,
            reset => '0',
            q => tmds_clk_post);

    hdmi_ser0: Gowin_DDR
        port map (
            din => tmds10_0,
            fclk => tmds_clock,
            pclk => hdmi_pixel_clock,
            reset => '0',
            q => tmds_0);
    hdmi_ser1: Gowin_DDR
        port map (
            din => tmds10_1,
            fclk => tmds_clock,
            pclk => hdmi_pixel_clock,
            reset => '0',
            q => tmds_1);
    hdmi_ser2: Gowin_DDR
        port map (
            din => tmds10_2,
            fclk => tmds_clock,
            pclk => hdmi_pixel_clock,
            reset => '0',
            q => tmds_2);

    hdmi_pll: tmds_pll port map(
        lock => pll_lock,
        clkout => tmds_clock,
        clkin => clock);

--    tiny_hdmi: tiny_hdmi_pll
--        port map (
--            clkout => tmds_clock,
--            clkin => clock
--        );

--    slow_pll_i: slow_pll
--        port map (
--            clkout => hdmi_pixel_clock,
--            clkin => clock);
--    tmds_clock <= clock;
    
    tmds_maker: tmds_div port map (
        clkout => hdmi_pixel_clock,
        hclkin => tmds_clock,
        resetn => '1'
    );

    hdmi_converter: entity work.hdmi generic map(
			t => "mux",
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
			vsync_out => test,
			r => "00101010",
			g => "01010101",
			b => "01110011");

    random: entity work.lfsr32 port map(
		clock => hdmi_pixel_clock,
		dout => random_data);
end Behavioral;

