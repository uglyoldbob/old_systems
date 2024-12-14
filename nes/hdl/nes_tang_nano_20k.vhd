library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity nes_tang_nano_20k is
   Generic(
        rambits: integer := 8);
   Port (
		clock: in std_logic;
		O_sdram_clk: out std_logic;
		O_sdram_cke: out std_logic;
		O_sdram_cs_n: out std_logic;
		O_sdram_cas_n: out std_logic;
		O_sdram_ras_n: out std_logic;
		O_sdram_wen_n: out std_logic;
		O_sdram_dqm: out std_logic_vector(3 downto 0);
		O_sdram_addr: out std_logic_vector(10 downto 0);
		O_sdram_ba: out std_logic_vector(1 downto 0);
		IO_sdram_dq: inout std_logic_vector(31 downto 0);
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
        test: out std_logic_vector(1 downto 0);
		test2: out std_logic_vector(1 downto 0);
        leds: out std_logic_vector(5 downto 0));
end nes_tang_nano_20k;

architecture Behavioral of nes_tang_nano_20k is
	signal button_clock: std_logic;

	signal rgb: std_logic_vector(23 downto 0);
	signal double_hdmi_pixel_clock: std_logic;
    signal hdmi_pixel_clock: std_logic;

    signal pll_lock: std_logic;
	signal pll_lock2: std_logic;

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
	signal tmds_clk_signal: std_logic;

    signal tmds_0_ddr: std_logic_vector(1 downto 0);
	signal tmds_1_ddr: std_logic_vector(1 downto 0);
	signal tmds_2_ddr: std_logic_vector(1 downto 0);

	signal tmds: std_logic_vector(2 downto 0);

	signal hdmi_row: std_logic_vector(10 downto 0);
	signal hdmi_column: std_logic_vector(11 downto 0);
	signal hdmi_hstart: std_logic;
	signal hdmi_vstart: std_logic;
	signal hdmi_pvalid: std_logic;

	signal crosshair_row: std_logic_vector(9 downto 0) := std_logic_vector(to_unsigned(80, 10));
	signal crosshair_column: std_logic_vector(10 downto 0) := std_logic_vector(to_unsigned(5, 11));

	signal debounce_buttona: std_logic;
	signal debounce_buttonb: std_logic;

	signal nes_clock: std_logic;
	signal nes_reset: std_logic;
	signal nes_oe: std_logic_vector(1 downto 0);
	signal nes_address: std_logic_vector(15 downto 0);

	signal ppu_pixel: std_logic_vector(23 downto 0);

	signal write_signal: std_logic;
	signal write_address: std_logic_vector(19 downto 0);
	signal write_value: std_logic_vector(7 downto 0);
	signal write_trigger: std_logic;
	signal write_rw: std_logic;
	signal write_cs: std_logic_vector(1 downto 0);

	signal hdmi_fifo_empty: std_logic;
	signal hdmi_fifo_full: std_logic;
	signal hdmi_fifo_write: std_logic;
	signal hdmi_fifo_read: std_logic;
	signal hdmi_pixel: std_logic_vector(23 downto 0);

    signal video_mode: std_logic_vector(2 downto 0):= (others => '0');

	signal sdram_wb_ack: std_logic;
	signal sdram_wb_d_miso: std_logic_vector(7 downto 0);
	signal sdram_wb_d_mosi: std_logic_vector(7 downto 0);
	signal sdram_wb_err: std_logic;
	signal sdram_wb_addr: std_logic_vector((rambits/8)+20 downto 0);
	signal sdram_wb_bte: std_logic_vector(1 downto 0);
	signal sdram_wb_cti: std_logic_vector(2 downto 0);
	signal sdram_wb_cyc: std_logic;
	signal sdram_wb_sel: std_logic_vector(0 downto 0);
	signal sdram_wb_stb: std_logic;
	signal sdram_wb_we: std_logic;

    component tmds_pll
		port (
			clkout: out std_logic;
			lock: out std_logic;
			clkin: in std_logic
		);
	end component;

	component gowin_nes_pll
		port (
			clkout: out std_logic;
			lock: out std_logic;
			clkoutd: out std_logic;
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
			tmds: in std_logic_vector(2 downto 0);
			tmds_clk: in std_logic;
			tmds_clk_n: out std_logic;
			tmds_clk_p: out std_logic;
			tmds_d_n: out std_logic_vector(2 downto 0);
			tmds_d_p: out std_logic_vector(2 downto 0));
	end component;

	component gowin_video_fifo
		port (
			Data: in std_logic_vector(23 downto 0);
			WrClk: in std_logic;
			RdClk: in std_logic;
			WrEn: in std_logic;
			RdEn: in std_logic;
			Q: out std_logic_vector(23 downto 0);
			Empty: out std_logic;
			Full: out std_logic
		);
	end component;

begin
    leds(5 downto 3) <= not video_mode;
    leds(2) <= '1';
    leds(1) <= not pll_lock and not pll_lock2;
    leds(0) <= not hdmi_hpd;

	test2(0) <= hdmi_fifo_write;
	test2(1) <= hdmi_fifo_read;

	tmds <= tmds_2 & tmds_1 & tmds_0;
	tmds_clk_signal <= tmds_clk_post(0);

	tmds_buf: test_hdmi_out port map (
		tmds_clk => tmds_clk_signal,
		tmds => tmds,
		tmds_clk_p => hdmi_ck_p,
		tmds_clk_n => hdmi_ck_n,
		tmds_d_p => hdmi_d_p,
		tmds_d_n => hdmi_d_n);

	hdmi_serclk: Gowin_DDR
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

	nes_pll: gowin_nes_pll port map (
		clkout => double_hdmi_pixel_clock,
		lock => pll_lock2,
		clkoutd => nes_clock,
		clkin => tmds_clock);

	process (double_hdmi_pixel_clock)
	begin
		if rising_edge(double_hdmi_pixel_clock) then
			hdmi_pixel_clock <= not hdmi_pixel_clock;
		end if;
	end process;

	hdmi_fifo: gowin_video_fifo port map (
		Data => ppu_pixel,
		WrClk => hdmi_pixel_clock,
		RdClk => hdmi_pixel_clock,
		WrEn => hdmi_fifo_write,
		RdEn => hdmi_fifo_read,
		Q => hdmi_pixel,
		Empty => hdmi_fifo_empty,
		Full => hdmi_fifo_full);

    hdmi_converter: entity work.hdmi2 generic map(
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
			reset => '0',
			pixel_clock => hdmi_pixel_clock,
			tmds_clock => tmds_clock,
			tmds_0 => tmds10_0,
			tmds_1 => tmds10_1,
			tmds_2 => tmds10_2,
			cec => hdmi_cec,
			i2c_scl => hdmi_i2c_scl,
			i2c_sda => hdmi_i2c_sda,
			hpd => hdmi_hpd,
			row_out => hdmi_row,
			column_out => hdmi_column,
			hstart => hdmi_hstart,
			vstart => hdmi_vstart,
			pvalid => hdmi_pvalid,
			test => test,
			r => rgb(23 downto 16),
			g => rgb(15 downto 8),
			b => rgb(7 downto 0));

	process (all)
	begin
        case video_mode is
            when "001" =>
                if hdmi_column < std_logic_vector(to_signed(512, 12)) and 
                    hdmi_pvalid = '1' then
                    hdmi_fifo_read <= '1';
                else
                    hdmi_fifo_read <= '0';
                end if;
            when others =>
                if hdmi_column >= std_logic_vector(to_signed(256, 12)) and
                    hdmi_column < std_logic_vector(to_signed(1024, 12)) and 
                    hdmi_pvalid = '1' then
                    hdmi_fifo_read <= '1';
                else
                    hdmi_fifo_read <= '0';
                end if;
        end case;
	end process;

	process (hdmi_pixel_clock)
	begin
		if rising_edge(hdmi_pixel_clock) then
            if debounce_buttona then
                case video_mode is
                    when "000" => video_mode <= "001";
                    when "001" => video_mode <= "010";
                    when "010" => video_mode <= "011";
                    when "011" => video_mode <= "100";
                    when "100" => video_mode <= "101";
                    when "101" => video_mode <= "110";
                    when "110" => video_mode <= "111";
                    when others => video_mode <= "000";
                end case;
            end if;
			rgb <= (others => '0');
            case video_mode is
                when "000" => 
                    if hdmi_column < std_logic_vector(to_signed(256, 12)) or hdmi_column > std_logic_vector(to_signed(1024, 12)) then
                        rgb <= (others => '0');
                    else
                        rgb <= hdmi_pixel;
                    end if;
                when "001" =>
                    if hdmi_column < std_logic_vector(to_signed(512, 12)) then
                        rgb <= (others => '0');
                    else
                        rgb <= hdmi_pixel;
                    end if;
                when "010" =>
                    if hdmi_fifo_write then
                        rgb(23 downto 16) <= (others => '1');
                    else
                        rgb(23 downto 16) <= (others => '0');
                    end if;
                    if hdmi_fifo_read then
                        rgb(15 downto 8) <= (others => '1');
                    else
                        rgb(15 downto 8) <= (others => '0');
                    end if;
                    rgb(7 downto 0) <= (others => '0');
                when "011" =>
                    rgb <= ppu_pixel;
                when "100" =>
                    if hdmi_fifo_full then
                        rgb(23 downto 16) <= (others => '1');
                    end if;
                    if hdmi_fifo_empty then
                        rgb(15 downto 8) <= (others => '1');
                    end if;
                    rgb(7 downto 0) <= (others => '0');
                when "101" =>
                    rgb(23 downto 16) <= (others => '1');
                    rgb(15 downto 8) <= (others => '0');
                    rgb(7 downto 0) <= (others => '0');
                when "110" =>
                    rgb(23 downto 16) <= (others => '0');
                    rgb(15 downto 8) <= (others => '1');
                    rgb(7 downto 0) <= (others => '0');
                when "111" =>
                    rgb(23 downto 16) <= (others => '0');
                    rgb(15 downto 8) <= (others => '0');
                    rgb(7 downto 0) <= (others => '1');
                when others =>
            end case;
		end if;
	end process;

	bc: entity work.large_divider generic map(bits => 20) port map(clock => hdmi_pixel_clock, ckout => button_clock);

	btn1: entity work.switch_debounce port map(
		slowclock => button_clock,
		clock => hdmi_pixel_clock,
		din => buttons(0),
		dout => debounce_buttona);

	btn2: entity work.switch_debounce port map(
		slowclock => button_clock,
		clock => hdmi_pixel_clock,
		din => buttons(1),
		dout => debounce_buttonb);

	write_signal <= '0';
	write_trigger <= '0';
	nes_reset <= '0';

	ram: entity work.gowin_sdram_interface port map(
		O_sdram_clk => O_sdram_clk,
		O_sdram_cke => O_sdram_cke,
		O_sdram_cs_n => O_sdram_cs_n,
		O_sdram_cas_n => O_sdram_cas_n,
		O_sdram_ras_n => O_sdram_ras_n,
		O_sdram_wen_n => O_sdram_wen_n,
		O_sdram_dqm => O_sdram_dqm,
		O_sdram_addr => O_sdram_addr,
		O_sdram_ba => O_sdram_ba,
		IO_sdram_dq => IO_sdram_dq,
		wb_ack => sdram_wb_ack,
		wb_d_miso => sdram_wb_d_miso,
		wb_d_mosi => sdram_wb_d_mosi,
		wb_err => sdram_wb_err,
		wb_addr => sdram_wb_addr,
		wb_bte => sdram_wb_bte,
		wb_cti => sdram_wb_cti,
		wb_cyc => sdram_wb_cyc,
		wb_sel => sdram_wb_sel,
		wb_stb => sdram_wb_stb,
		wb_we => sdram_wb_we);

	nes: entity work.nes generic map(
		random_noise => '1') port map (
		rom_wb_ack => sdram_wb_ack,
		rom_wb_d_miso => sdram_wb_d_miso,
		rom_wb_d_mosi => sdram_wb_d_mosi,
		rom_wb_err => sdram_wb_err,
		rom_wb_addr => sdram_wb_addr((rambits/8)+20 downto 0),
		rom_wb_bte => sdram_wb_bte,
		rom_wb_cti => sdram_wb_cti,
		rom_wb_cyc => sdram_wb_cyc,
		rom_wb_sel => sdram_wb_sel,
		rom_wb_stb => sdram_wb_stb,
		rom_wb_we => sdram_wb_we,
		hdmi_pixel_out => ppu_pixel,
		hdmi_row => hdmi_row,
		hdmi_column => hdmi_column,
		hdmi_pvalid => hdmi_pvalid,
		hdmi_valid_out => hdmi_fifo_write,
		hdmi_line_ready => hdmi_hstart,
		reset => nes_reset,
		cpu_oe => nes_oe,
		cpu_memory_address => nes_address,
		fast_clock => hdmi_pixel_clock,
		clock => nes_clock,
		hdmi_vsync => hdmi_vstart);
end Behavioral;

