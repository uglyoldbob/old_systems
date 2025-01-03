library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use std.textio.all;
use ieee.std_logic_textio.all;
use ieee.numeric_std.all;
use std.env.finish;

entity nestb is
    Port (whocares: out std_logic;
	   cpu_oe: out std_logic_vector(1 downto 0);
		cpu_memory_address: out std_logic_vector(15 downto 0);
		cs_out: out std_logic_vector(3 downto 0);
		otherstuff: out std_logic_vector(15 downto 0));
end nestb;

architecture Behavioral of nestb is

type AUDIO_TYPE is array(1 downto 0) of std_logic_vector(15 downto 0);
signal audio: AUDIO_TYPE;

type NES_TEST_ENTRY is
	record
		pc: std_logic_vector(15 downto 0);
		num_bytes: std_logic_vector(1 downto 0);
		a: std_logic_vector(7 downto 0);
		x: std_logic_vector(7 downto 0);
		y: std_logic_vector(7 downto 0);
		p: std_logic_vector(7 downto 0);
		sp: std_logic_vector(7 downto 0);
		cycle: std_logic_vector(14 downto 0);
		dummy: std_logic_vector(7 downto 0);
	end record;
type NES_TEST_ENTRIES is array (integer range<>) of NES_TEST_ENTRY;

impure function GetNestestResults (FileName : in string; entries: integer) return NES_TEST_ENTRIES is
	variable results : NES_TEST_ENTRIES(0 to entries-1);
	FILE romfile : text is in FileName;
	variable open_status :FILE_OPEN_STATUS;
	file     infile      :text;
	variable RomFileLine : line;
	begin
		file_open(open_status, infile, filename, read_mode);
		for i in 0 to entries-1 loop
			readline(romfile, RomFileLine);
			hread(RomFileLine, results(i).pc);
			hread(RomFileLine, results(i).num_bytes);
			hread(RomFileLine, results(i).a);
			hread(RomFileLine, results(i).x);
			hread(RomFileLine, results(i).y);
			hread(RomFileLine, results(i).p);
			hread(RomFileLine, results(i).sp);
			hread(RomFileLine, results(i).cycle);
			hread(RomFileLine, results(i).dummy);
		end loop;
		return results;
	end function;
	
	signal run_benches: std_logic_vector(2 downto 0) := "100";

	signal test_signals: NES_TEST_ENTRIES(0 to 8990) := GetNestestResults("nestest.txt", 8991);
	signal cpu_clock: std_logic := '0';
	signal cpu_reset: std_logic := '0';
	signal cpu_address: std_logic_vector(15 downto 0);
	signal cpu_dout: std_logic_vector(7 downto 0);
	signal cpu_din: std_logic_vector(7 downto 0);
	
	signal cpu_a: std_logic_vector(7 downto 0);
	signal cpu_x: std_logic_vector(7 downto 0);
	signal cpu_y: std_logic_vector(7 downto 0);
	signal cpu_pc: std_logic_vector(15 downto 0);
	signal cpu_sp: std_logic_vector(7 downto 0);
	signal cpu_flags: std_logic_vector(7 downto 0);
	signal cpu_cycle: std_logic_vector(14 downto 0);
	signal cpu_subcycle: std_logic_vector(3 downto 0);
	
	signal ppu_pixel: std_logic_vector(23 downto 0);
	signal ppu_pixel_valid: std_logic;
	
	signal cpu_memory_clock: std_logic;
	signal cpu_instruction: std_logic;
	signal instruction_check: std_logic;
	
	signal hdmi_pixel_clock: std_logic := '0';
	signal hdmi_tmds_clock: std_logic := '0';
	signal hdmi_tmds_0: std_logic_vector(9 downto 0);
	signal hdmi_tmds_1: std_logic_vector(9 downto 0);
	signal hdmi_tmds_2: std_logic_vector(9 downto 0);
	signal hdmi_d_0_p: std_logic;
	signal hdmi_d_0_n: std_logic;
	signal hdmi_d_1_p: std_logic;
	signal hdmi_d_1_n: std_logic;
	signal hdmi_d_2_p: std_logic;
	signal hdmi_d_2_n: std_logic;
	signal hdmi_ck_p: std_logic;
	signal hdmi_ck_n: std_logic;
	signal hdmi_cec: std_logic;
	signal hdmi_i2c_scl: std_logic;
	signal hdmi_i2c_sda: std_logic;
	signal hdmi_hpd: std_logic;
	
	signal hdmi_row: std_logic_vector(10 downto 0);
	signal hdmi_column: std_logic_vector(11 downto 0);
	signal hdmi_hstart: std_logic;
	signal hdmi_vstart: std_logic;
	signal hdmi_vblank: std_logic;
	signal hdmi_pvalid: std_logic;
	
	signal hdmi1_tmds: std_logic_vector(2 downto 0);
	signal hdmi1_clk: std_logic;
	signal hdmi1_cx: std_logic_vector(10 downto 0);
	signal hdmi1_cy: std_logic_vector(9 downto 0);
	signal hdmi1_frame_width: std_logic_vector(10 downto 0);
	signal hdmi1_frame_height: std_logic_vector(9 downto 0);
	signal hdmi1_screen_width: std_logic_vector(10 downto 0);
	signal hdmi1_screen_height: std_logic_vector(9 downto 0);
	
	signal rom_addr_mask: std_logic_vector(21 downto 0);
	signal rom_wb_ack: std_logic;
	signal rom_wb_d_miso: std_logic_vector(15 downto 0);
	signal rom_wb_d_mosi: std_logic_vector(15 downto 0);
	signal rom_wb_err: std_logic;
	signal rom_wb_addr: std_logic_vector(21 downto 0);
	signal rom_wb_bte: std_logic_vector(1 downto 0);
	signal rom_wb_cti: std_logic_vector(2 downto 0);
	signal rom_wb_cyc: std_logic;
	signal rom_wb_sel: std_logic_vector(1 downto 0);
	signal rom_wb_stb: std_logic;
	signal rom_wb_we: std_logic;
	
	type RAM_ARRAY is array (2**21 downto 0) of std_logic_vector (7 downto 0);
	signal rom : RAM_ARRAY;
	FILE romfile : text;
	
	signal random_data: std_logic_vector(31 downto 0);
	signal rgb: std_logic_vector(23 downto 0);
	
	signal clk27: std_logic := '0';
	signal fast_cpu_clock: std_logic := '0';
	signal double_fast_cpu_clock: std_logic := '0';
	signal cpu_clock_counter: integer range 0 to 3 := 0;
	
	signal soft_cpu_clock: std_logic := '0';
	
	constant NESTEST_ADDRESS   : integer := 16#3ffc#;
begin
	hdmi_pixel_clock <= fast_cpu_clock;
	hdmi_tmds_clock <= not hdmi_tmds_clock after 1.347 ns;
	double_fast_cpu_clock <= not double_fast_cpu_clock after 3.367 ns;
	
	process (double_fast_cpu_clock)
	begin
		if rising_edge(double_fast_cpu_clock) then
			fast_cpu_clock <= not fast_cpu_clock;
			cpu_clock_counter <= cpu_clock_counter + 1;
			if cpu_clock_counter = 2 then
				cpu_clock_counter <= 0;
				cpu_clock <= not cpu_clock;
			end if;
		end if;
	end process;
	
	process (fast_cpu_clock)
	begin
		if rising_edge(fast_cpu_clock) then
			soft_cpu_clock <= not soft_cpu_clock;
		end if;
	end process;
	
	clk27 <= not clk27 after 18.519 ns;
	otherstuff <= cpu_address;
	cpu_memory_address <= cpu_address;
	
	hdmi_i2c_scl <= 'H';
	hdmi_i2c_sda <= 'H';
	
	random: entity work.lfsr32 port map(
		clock => hdmi_pixel_clock,
		dout => random_data);
	
	hdmi: entity work.hdmi generic map (DVI_OUTPUT => '1', VIDEO_ID_CODE => 4)
		port map(
			clk_pixel_x5 => hdmi_tmds_clock,
			clk_pixel => hdmi_pixel_clock,
			clk_audio => '0',
			reset => cpu_reset,
			rgb => random_data(23 downto 0),
			audio_sample_word => audio,
			tmds => hdmi1_tmds,
			tmds_clock => hdmi1_clk,
			cx => hdmi1_cx,
			cy => hdmi1_cy,
			frame_width => hdmi1_frame_width,
			frame_height => hdmi1_frame_height,
			screen_width => hdmi1_screen_width,
			screen_height => hdmi1_screen_height);
	
	hdmi2: entity work.hdmi2 generic map(
		hsync_polarity => '1',
		vsync_polarity => '1',
		h => 1280,
		v => 720,
		hblank_width => 370,
		hsync_porch => 220,
		hsync_width => 40,
		vblank_width => 30,
		vsync_porch => 20,
		vsync_width => 5) port map (
		reset => cpu_reset,
		tmds_0 => hdmi_tmds_0,
		tmds_1 => hdmi_tmds_1,
		tmds_2 => hdmi_tmds_2,
		cec => hdmi_cec,
		i2c_scl => hdmi_i2c_scl,
		i2c_sda => hdmi_i2c_sda,
		hpd => hdmi_hpd,
		pixel_clock => hdmi_pixel_clock,
		tmds_clock => hdmi_tmds_clock,
		row_out => hdmi_row,
		column_out => hdmi_column,
		hstart => hdmi_hstart,
		vstart => hdmi_vstart,
		vblank_out => hdmi_vblank,
		pvalid => hdmi_pvalid,
		r => rgb(23 downto 16),
		g => rgb(15 downto 8),
		b => rgb(7 downto 0)
	);
	
	process (hdmi_pixel_clock)
	begin
		if rising_edge(hdmi_pixel_clock) then
			if hdmi_column = std_logic_vector(to_signed(1278, 12)) or hdmi_column = std_logic_vector(to_signed(0, 12)) then
				rgb <= "000000000000000000000000";
			elsif hdmi_column = std_logic_vector(to_signed(1279, 12)) or hdmi_column = std_logic_vector(to_signed(1, 12)) then
				rgb <= "000000000000000011111111";
			elsif hdmi_column = std_logic_vector(to_signed(1280, 12)) or hdmi_column = std_logic_vector(to_signed(2, 12)) then
				rgb <= "111111111111111111111111";
			elsif hdmi_pvalid then
				rgb <= random_data(23 downto 0);
			else
				rgb <= "000000001111111100000000";
			end if;
		end if;
	end process;
	
	nes: entity work.nes generic map(
		clockbuf => "none",
		sim => '1',
		ramtype => "wishbone",
		rambits => 4,
		random_noise => '1') port map (
		ignore_sync => '0',
		rom_wb_ack => rom_wb_ack,
		rom_wb_d_miso => rom_wb_d_miso,
		rom_wb_d_mosi => rom_wb_d_mosi,
		rom_wb_err => rom_wb_err,
		rom_wb_addr => rom_wb_addr,
		rom_wb_bte => rom_wb_bte,
		rom_wb_cti => rom_wb_cti,
		rom_wb_cyc => rom_wb_cyc,
		rom_wb_sel => rom_wb_sel,
		rom_wb_stb => rom_wb_stb,
		rom_wb_we => rom_wb_we,
		hdmi_pixel_out => ppu_pixel,
		hdmi_vsync => hdmi_vblank,
		hdmi_row => hdmi_row,
		hdmi_column => hdmi_column,
		hdmi_pvalid => hdmi_pvalid,
		hdmi_line_ready => hdmi_hstart,
		d_memory_clock => cpu_memory_clock,
		d_a => cpu_a,
		d_x => cpu_x,
		d_y => cpu_y,
		d_pc => cpu_pc,
		d_sp => cpu_sp,
		d_flags => cpu_flags,
		d_subcycle => cpu_subcycle,
		d_cycle => cpu_cycle,
		instruction_toggle_out => cpu_instruction,
		reset => cpu_reset,
		cpu_oe => cpu_oe,
		cpu_memory_address => cpu_address,
		cs_out => cs_out,
		whocares => whocares,
		soft_cpu_clock => soft_cpu_clock,
		clock => fast_cpu_clock
		);
		
	process (hdmi_pixel_clock)
	begin
		if rising_edge(hdmi_pixel_clock) then
			if rom_wb_cyc and rom_wb_stb then
				rom_wb_d_miso(7 downto 0) <= rom(to_integer(unsigned((rom_wb_addr and ("0" & rom_addr_mask(21 downto 1))) & '0')));
				rom_wb_d_miso(15 downto 8) <= rom(to_integer(unsigned((rom_wb_addr and ("0" & rom_addr_mask(21 downto 1))) & '1')));
			end if;
		end if;
	end process;
	
	rom_wb_ack <= rom_wb_cyc and rom_wb_stb;
	rom_wb_err <= '0';

	process
		variable RomFileLine : line;
		variable rom_value: std_logic_vector(7 downto 0);
		variable i: integer;
		variable size: integer;
	begin
		cpu_reset <= '1', '0' after 100ns;
		
		wait until rising_edge(cpu_clock);
		
		if run_benches(0) or run_benches(1) then
			file_open(romfile, "nestest_prg_rom.txt", read_mode);
			i := 0;
			while not endfile(romfile) loop
				readline(romfile, RomFileLine);
				hread(RomFileLine, rom_value);
				rom(i) <= rom_value;
				i := i + 1;
			end loop;
			report("Read " & to_string(i) & " bytes");
			size := i;
			i := 0;
			rom_addr_mask <= std_logic_vector(to_unsigned(size - 1, 22));
			--provide a start address mod for nestest rom
			if run_benches(0) then
				rom(NESTEST_ADDRESS) <= x"00";
				wait until rising_edge(cpu_clock);
			end if;
		end if;

		if run_benches(0) then
			report "Running basic nestest to check cpu";
			instruction_check <= '0';
			wait until rising_edge(cpu_clock);
			for i in 0 to 8990 loop
				wait until cpu_instruction /= instruction_check or cpu_subcycle = "1111";
				assert cpu_a = test_signals(i).a
					report "Failure of A on instruction " & to_string(i+1) & ", Expected(" & to_hstring(test_signals(i).a) & ") got (" & to_hstring(cpu_a) & ")"
					severity failure;
				assert cpu_x = test_signals(i).x
					report "Failure of X on instruction " & to_string(i+1) & ", Expected(" & to_hstring(test_signals(i).x) & ") got (" & to_hstring(cpu_x) & ")"
					severity failure;
				assert cpu_y = test_signals(i).y
					report "Failure of Y on instruction " & to_string(i+1) & ", Expected(" & to_hstring(test_signals(i).y) & ") got (" & to_hstring(cpu_y) & ")"
					severity failure;
				assert cpu_sp = test_signals(i).sp
					report "Failure of SP on instruction " & to_string(i+1) & ", Expected(" & to_hstring(test_signals(i).sp) & ") got (" & to_hstring(cpu_sp) & ")"
					severity failure;
				assert cpu_flags = test_signals(i).p
					report "Failure of FLAGS on instruction " & to_string(i+1) & ", Expected(" & to_hstring(test_signals(i).p) & ") got (" & to_hstring(cpu_flags) & ")"
					severity failure;
				instruction_check <= cpu_instruction;
				assert cpu_pc = test_signals(i).pc
					report "Failure of PC on instruction " & to_string(i+1) & ", Expected(" & to_hstring(test_signals(i).pc) & ") got (" & to_hstring(cpu_pc) & ")"
					severity failure;
				assert cpu_cycle = test_signals(i).cycle
					report "Failure of cycle on instruction " & to_string(i+1) & ", Expected(0x" & to_hstring(test_signals(i).cycle) & ") got (0x" & to_hstring(cpu_cycle) & ")"
					severity failure;
			end loop;
		end if;
		
		if run_benches(1) then
			--restore original value for nestest rom
			rom(NESTEST_ADDRESS) <= x"04";
			cpu_reset <= '1';
			report "Running normal nestest to check cpu";
			wait for 100ns;
			cpu_reset <= '0';
			instruction_check <= '0';
			for i in 0 to 8990 loop
				wait until cpu_instruction /= instruction_check or cpu_subcycle = "1111";
				instruction_check <= cpu_instruction;
			end loop;
			wait for 80000000ns;
			report "Just checking" severity failure;
		end if;
		
		if run_benches(2) then
			wait for 800us;
			report "Pause for examination" severity failure;
			wait for 160000000ns;
			report "Just checking" severity failure;
		end if;
		
		report "Test complete";
		finish;
	end process;
end Behavioral;

