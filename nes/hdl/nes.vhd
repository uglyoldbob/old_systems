library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity nes is
	Generic (
		ramtype: string := "sram";
		random_noise: in std_logic := '1';
		unified_ram: std_logic := '0');
   Port (
		ignore_sync: in std_logic := '0';
		write_signal: in std_logic := '0';
		write_address: in std_logic_vector(19 downto 0) := (others=>'0');
		write_value: in std_logic_vector(7 downto 0) := (others=>'0');
		write_trigger: in std_logic := '0';
		write_rw: in std_logic;
		write_cs: in std_logic_vector(1 downto 0) := (others=>'0');
		asdf: out std_logic;
		hdmi_pixel_out: out std_logic_vector(23 downto 0);
		hdmi_vsync: in std_logic;
		hdmi_row: in std_logic_vector(10 downto 0);
		hdmi_column: in std_logic_vector(11 downto 0);
		hdmi_valid_out: out std_logic;
		hdmi_pvalid: in std_logic;
		hdmi_line_done: out std_logic;
		hdmi_line_ready: in std_logic;
		
		d_a: out std_logic_vector(7 downto 0) := x"00";
		d_x: out std_logic_vector(7 downto 0) := x"00";
		d_y: out std_logic_vector(7 downto 0) := x"00";
		d_pc: out std_logic_vector(15 downto 0);
		d_sp: out std_logic_vector(7 downto 0) := x"fd";
		d_flags: out std_logic_vector(7 downto 0) := x"24";
		d_memory_clock: out std_logic;
		d_subcycle: out std_logic_vector(3 downto 0);
		d_cycle: out std_logic_vector(14 downto 0);
		instruction_toggle_out: out std_logic;
		reset: in std_logic;
	   fast_clock: in std_logic;
		clock: in std_logic; --fast_clock divided by 3
		cpu_oe: out std_logic_vector(1 downto 0);
		cpu_memory_address: out std_logic_vector(15 downto 0);
	   whocares: out std_logic;
		cs_out: out std_logic_vector(3 downto 0);
		otherstuff: out std_logic_vector(15 downto 0));
end nes;

architecture Behavioral of nes is

	signal line0_address: std_logic_vector(7 downto 0);
	signal line0_rw: std_logic;
	signal line0_dout: std_logic_vector(23 downto 0);
	signal line0_dout_valid: std_logic;
	signal line0_din: std_logic_vector(23 downto 0);
	
	signal line1_address: std_logic_vector(7 downto 0);
	signal line1_rw: std_logic;
	signal line1_dout: std_logic_vector(23 downto 0);
	signal line1_dout_valid: std_logic;
	signal line1_din: std_logic_vector(23 downto 0);
	
	signal line2_address: std_logic_vector(7 downto 0);
	signal line2_rw: std_logic;
	signal line2_dout: std_logic_vector(23 downto 0);
	signal line2_dout_valid: std_logic;
	signal line2_din: std_logic_vector(23 downto 0);
	
	signal line_counter: std_logic_vector(1 downto 0) := (others => '0');
	
	signal line_out_0_address: std_logic_vector(9 downto 0);
	signal line_out_0_rw: std_logic;
	signal line_out_0_dout: std_logic_vector(23 downto 0);
	signal line_out_0_dout_valid: std_logic;
	signal line_out_0_din: std_logic_vector(23 downto 0);

	signal line_out_1_address: std_logic_vector(9 downto 0);
	signal line_out_1_rw: std_logic;
	signal line_out_1_dout: std_logic_vector(23 downto 0);
	signal line_out_1_dout_valid: std_logic;
	signal line_out_1_din: std_logic_vector(23 downto 0);

	signal line_out_2_address: std_logic_vector(9 downto 0);
	signal line_out_2_rw: std_logic;
	signal line_out_2_dout: std_logic_vector(23 downto 0);
	signal line_out_2_dout_valid: std_logic;
	signal line_out_2_din: std_logic_vector(23 downto 0);

	signal line_out_3_address: std_logic_vector(9 downto 0);
	signal line_out_3_rw: std_logic;
	signal line_out_3_dout: std_logic_vector(23 downto 0);
	signal line_out_3_dout_valid: std_logic;
	signal line_out_3_din: std_logic_vector(23 downto 0);

	signal line_out_4_address: std_logic_vector(9 downto 0);
	signal line_out_4_rw: std_logic;
	signal line_out_4_dout: std_logic_vector(23 downto 0);
	signal line_out_4_dout_valid: std_logic;
	signal line_out_4_din: std_logic_vector(23 downto 0);
	
	signal line_out_5_address: std_logic_vector(9 downto 0);
	signal line_out_5_rw: std_logic;
	signal line_out_5_dout: std_logic_vector(23 downto 0);
	signal line_out_5_dout_valid: std_logic;
	signal line_out_5_din: std_logic_vector(23 downto 0);

	signal line_out_counter: integer range 0 to 5 := 0;

	
	signal kernel_a: std_logic_vector(23 downto 0);
	signal kernel_b: std_logic_vector(23 downto 0);
	signal kernel_c: std_logic_vector(23 downto 0);
	signal kernel_d: std_logic_vector(23 downto 0);
	signal kernel_e: std_logic_vector(23 downto 0);
	signal kernel_f: std_logic_vector(23 downto 0);
	signal kernel_g: std_logic_vector(23 downto 0);
	signal kernel_h: std_logic_vector(23 downto 0);
	signal kernel_i: std_logic_vector(23 downto 0);
	
	signal kernel_out_a: std_logic_vector(23 downto 0);
	signal kernel_out_b: std_logic_vector(23 downto 0);
	signal kernel_out_c: std_logic_vector(23 downto 0);
	signal kernel_out_d: std_logic_vector(23 downto 0);
	signal kernel_out_e: std_logic_vector(23 downto 0);
	signal kernel_out_f: std_logic_vector(23 downto 0);
	signal kernel_out_g: std_logic_vector(23 downto 0);
	signal kernel_out_h: std_logic_vector(23 downto 0);
	signal kernel_out_i: std_logic_vector(23 downto 0);

	signal cpu_address: std_logic_vector(15 downto 0);
	signal cpu_dout: std_logic_vector(7 downto 0);
	signal cpu_din: std_logic_vector(7 downto 0);
	signal cpu_din_ready: std_logic;
	signal cpu_dready: std_logic;
	signal cpu_rw: std_logic;
	signal cpu_memory_clock: std_logic;
	signal memory_clock: std_logic;
	
	signal cpu_sram_din: std_logic_vector(7 downto 0);
	signal cpu_sram_dout: std_logic_vector(7 downto 0);
	signal cpu_ram_cs: std_logic;
	
	signal cpu_ppu_cs: std_logic;
	signal cpu_ppu_din: std_logic_vector(7 downto 0);
	signal cpu_ppu_dout: std_logic_vector(7 downto 0);
	
	signal ppu_int: std_logic;
	signal ppu_ale: std_logic;
	signal ppu_ad: std_logic_vector(7 downto 0);
	signal ppu_a: std_logic_vector(5 downto 0);
	signal ppu_din: std_logic_vector(7 downto 0);
	signal ppu_rd: std_logic;
	signal ppu_wr: std_logic;
	signal ppu_clock: std_logic;
	
	signal ppu_hstart_trigger: std_logic;
	signal ppu_vstart_trigger: std_logic;
	signal ppu_pixel_trigger: std_logic;
	signal ppu_clock_delay: std_logic;
	signal ppu_pixel_valid: std_logic;
	signal ppu_hstart: std_logic;
	signal ppu_vstart: std_logic;
	signal ppu_hstart_delay: std_logic;
	signal ppu_vstart_delay: std_logic;
	signal ppu_row: std_logic_vector(8 downto 0);
	signal ppu_column: std_logic_vector(8 downto 0);
	signal ppu_subpixel: std_logic_vector(3 downto 0);
	signal ppu_subpixel_process: std_logic_vector(3 downto 0);
	
	signal ppu_column_delay: std_logic_vector(8 downto 0);
	signal ppu_column_change: std_logic;
	signal ppu_last_column_trigger: std_logic;
	signal ppu_last_column_count: std_logic_vector(3 downto 0) := (others => '0');
	signal ppu_last_row_trigger: std_logic;
	signal ppu_last_row_count: std_logic_vector(12 downto 0) := (others => '0');
	signal ppu_process_column: std_logic_vector(8 downto 0) := (others => '0');
	signal ppu_process_row: std_logic_vector(8 downto 0) := (others => '0');
	signal ppu_last_row_pixel_trigger: std_logic;
	signal ppu_first_row_skip: std_logic := '0';
	signal ppu_first_column_skip: std_logic := '0';
	signal ppu_border: std_logic_vector(2 downto 0);
	constant BORDER_LEFT_RIGHT: integer := 0;
	constant BORDER_UP: integer := 1;
	constant BORDER_DOWN: integer := 2;
	signal ppu_rescale_row: std_logic;
	signal ppu_rescale_column: std_logic;
	signal ppu_rescale_trigger: std_logic;
	signal ppu_rescale_out_column1: integer range 0 to 767;
	signal ppu_rescale_out_column2: integer range 0 to 767;
	signal ppu_rescale_out_column3: integer range 0 to 767;
	
	signal cpu_apu_cs: std_logic;
	
	signal cpu_cartridge_cs: std_logic;
	signal cpu_cartridge_din: std_logic_vector(7 downto 0);
	signal cpu_cartridge_din_ready: std_logic;

	signal pause: std_logic;
	signal fsync_pause: std_logic;
	signal hdmi_vsync_delay: std_logic;
	signal hdmi_vsync_trigger: std_logic;
	signal ppu_vsync_sync: std_logic;
	
	signal ppu_pixel: std_logic_vector(23 downto 0);
	signal pixel_out: std_logic_vector(23 downto 0);
	
	signal hdmi_valid_calc: std_logic;
	signal hdmi_valid_calc2: std_logic;
	signal hdmi_column_calc: integer range 0 to 767;
	signal hdmi_row_calc: integer range 0 to 5;
	signal hdmi_line_done_sig: std_logic;
	signal hdmi_ppu_column: std_logic_vector(8 downto 0);
	signal hdmi_output_row: integer range 0 to 5;
	
	signal reset_sync: std_logic;
	signal reset_chain: std_logic;
begin
	whocares <= clock;
	otherstuff <= cpu_address;
	cpu_memory_address <= cpu_address;
	cs_out <= cpu_ram_cs & cpu_ppu_cs & cpu_apu_cs & cpu_cartridge_cs;
	hdmi_line_done <= hdmi_line_done_sig;
	
	d_memory_clock <= memory_clock;
	pause <= write_signal or (cpu_memory_clock and not cpu_din_ready) or (fsync_pause and not ignore_sync);
	
	process (clock)
	begin
		if rising_edge(clock) then
			reset_sync <= reset_chain;
			reset_chain <= reset;
		end if;
	end process;
	
	process (all)
	begin
		if cpu_address(15 downto 13) = "000" then
			cpu_ram_cs <= '1';
		else
			cpu_ram_cs <= '0';
		end if;
		if not cpu_address(15) and not cpu_address(14) and (cpu_address(13) or cpu_address(12)) then
			cpu_ppu_cs <= '1';
		else
			cpu_ppu_cs <= '0';
		end if;
		if cpu_address(15 downto 5) = "00000000000" then
			cpu_apu_cs <= '1';
		else
			cpu_apu_cs <= '0';
		end if;
		cpu_cartridge_cs <= not (cpu_ram_cs or cpu_ppu_cs or cpu_apu_cs);
		if cpu_ram_cs then
			cpu_din <= cpu_sram_dout;
		elsif cpu_ppu_cs then
			cpu_din <= cpu_ppu_din;
		elsif cpu_cartridge_cs then
			cpu_din <= cpu_cartridge_din;
		else
			cpu_din <= "00000000";
		end if;
		if cpu_cartridge_cs then
			cpu_din_ready <= cpu_cartridge_din_ready;
		else
			cpu_din_ready <= '1';
		end if;
	end process;
	
	process (clock)
	begin
		if rising_edge(clock) then
			if write_signal then
				memory_clock <= write_trigger;
			else
				memory_clock <= cpu_memory_clock;
			end if;
		end if;
	end process;
	
	process (reset_sync, memory_clock)
	begin
		if reset_sync = '1' then
			cpu_dready <= '0';
		elsif rising_edge(memory_clock) then
			if cpu_ram_cs or cpu_cartridge_cs then
				cpu_dready <= '1';
			end if;
		end if;
	end process;
	
	ddrtest: entity work.ddr generic map (t => "mux")
		port map(
			din => cpu_address(1 downto 0),
			dout => asdf,
			clock => clock);
	
	ram_nonunified: if (unified_ram = '0' and ramtype = "sram") generate
		cpu_ram: entity work.clocked_sram generic map (
			bits => 11
		) port map (
			clock => memory_clock,
			cs => cpu_ram_cs,
			address => cpu_address(10 downto 0),
			rw => cpu_rw,
			din => cpu_dout,
			dout => cpu_sram_dout
		);
	end generate;
	
	ppu_pixel_trigger <= ppu_clock and not ppu_clock_delay and ppu_pixel_valid;
	process (all)
	begin
		if ppu_last_column_count = "0010" and ppu_process_column > std_logic_vector(to_unsigned(252, 9)) 
			and ppu_process_column < std_logic_vector(to_unsigned(255, 9)) then
			ppu_last_column_trigger <= '1';
		else
			ppu_last_column_trigger <= '0';
		end if;
		if not ppu_last_row_trigger then
			ppu_subpixel_process <= ppu_subpixel;
		else
			ppu_subpixel_process <= ppu_last_row_count(3 downto 0);
		end if;
		if ppu_row > std_logic_vector(to_unsigned(0, 9)) and ppu_row < std_logic_vector(to_unsigned(241, 9)) then
			ppu_rescale_row <= '1';
		else
			ppu_rescale_row <= '0';
		end if;
		if ppu_column > std_logic_vector(to_unsigned(1, 9)) and ppu_column < std_logic_vector(to_unsigned(258, 9)) then
			ppu_rescale_column <= '1';
		else
			ppu_rescale_column <= '0';
		end if;
		if ppu_process_column /= std_logic_vector(to_unsigned(510, 9)) and ppu_process_column /= std_logic_vector(to_unsigned(255, 9)) then
			ppu_border(BORDER_LEFT_RIGHT) <= '1';
		else
			ppu_border(BORDER_LEFT_RIGHT) <= '0';
		end if;
		if ppu_row /= std_logic_vector(to_unsigned(1, 9)) then
			ppu_border(BORDER_UP) <= '1';
		else
			ppu_border(BORDER_UP) <= '0';
		end if;
		if ppu_row < std_logic_vector(to_unsigned(240, 9)) then
			ppu_border(BORDER_DOWN) <= '1';
		else
			ppu_border(BORDER_DOWN) <= '0';
		end if;
		if ppu_rescale_column = '1' and ppu_rescale_row = '1' and ppu_subpixel_process = "0010" then
			ppu_rescale_trigger <=  '1';
		else
			ppu_rescale_trigger <= '0';
		end if;
		if ppu_column < std_logic_vector(to_unsigned(256, 9)) and ppu_row = std_logic_vector(to_unsigned(240, 9)) then
			ppu_last_row_trigger <= '1';
		else
			ppu_last_row_trigger <= '0';
		end if;
		if ppu_column_delay /= ppu_column then
			ppu_column_change <= '1';
		else
			ppu_column_change <= '0';
		end if;
		if ppu_column_change = '1' and ppu_column < std_logic_vector(to_unsigned(256, 9)) then
			ppu_last_row_pixel_trigger <= '1';
		else
			ppu_last_row_pixel_trigger <= '0';
		end if;
		case hdmi_output_row is
			when 0 => hdmi_pixel_out <= line_out_0_dout;
			when 1 => hdmi_pixel_out <= line_out_1_dout;
			when 2 => hdmi_pixel_out <= line_out_2_dout;
			when 3 => hdmi_pixel_out <= line_out_3_dout;
			when 4 => hdmi_pixel_out <= line_out_4_dout;
			when others => hdmi_pixel_out <= line_out_5_dout;
		end case;
	end process;
	
	process (fast_clock)
	begin
		if rising_edge(fast_clock) then
			hdmi_valid_calc2 <= hdmi_valid_calc;
			hdmi_valid_out <= hdmi_valid_calc2;
			hdmi_ppu_column <= ppu_column;
			hdmi_output_row <= hdmi_row_calc;
			if hdmi_ppu_column < std_logic_vector(to_unsigned(256, 9)) and
				ppu_process_row > std_logic_vector(to_unsigned(0, 9)) and 
				ppu_process_row < std_logic_vector(to_unsigned(240, 9)) and
				ppu_subpixel_process > std_logic_vector(to_unsigned(2, 4)) and 
				ppu_subpixel_process < std_logic_vector(to_unsigned(12, 4)) then
				hdmi_valid_calc <= '1';
			else
				hdmi_valid_calc <= '0';
			end if;
			if ppu_row > std_logic_vector(to_unsigned(3, 9)) and
				ppu_row < std_logic_vector(to_unsigned(243, 9)) and
				ppu_column = std_logic_vector(to_unsigned(258,9)) and 
				ppu_subpixel_process = std_logic_vector(to_unsigned(12,4)) then
				hdmi_line_done_sig <= '1';
			else
				hdmi_line_done_sig <= '0';
			end if;
			ppu_column_delay <= ppu_column;
			if ppu_vstart_trigger = '1' then-- and ppu_process_row = std_logic_vector(to_unsigned(3, 9)) then
				ppu_vsync_sync <= '1';
			else
				ppu_vsync_sync <= '0';
			end if;
			ppu_vstart_trigger <= ppu_vstart and not ppu_vstart_delay;
			ppu_vstart_delay <= ppu_vstart;
			hdmi_vsync_delay <= hdmi_vsync;
			hdmi_vsync_trigger <= hdmi_vsync and not hdmi_vsync_delay;
			if ppu_vstart_trigger then
				hdmi_column_calc <= 0;
				hdmi_row_calc <= 0;
				ppu_first_row_skip <= '0';
			elsif ppu_row = std_logic_vector(to_unsigned(1, 9)) then
				ppu_first_row_skip <= '1';
			end if;
			if ppu_hstart_trigger then
				ppu_rescale_out_column1 <= 0;
				ppu_rescale_out_column2 <= 1;
				ppu_rescale_out_column3 <= 2;
				ppu_first_column_skip <= '0';
			elsif ppu_subpixel = "1100" then
				ppu_first_column_skip <= '1';
			end if;
			ppu_hstart_trigger <= ppu_hstart and not ppu_hstart_delay;
			ppu_clock_delay <= ppu_clock;
			ppu_hstart_delay <= ppu_hstart;
			if ppu_hstart_trigger = '1' or (ppu_last_row_trigger = '1' and ppu_last_row_count = "0000000000000" and ppu_row = std_logic_vector(to_unsigned(241, 9))) then
				case line_counter is
					when "00" => line_counter <= "01";
					when "01" => line_counter <= "10";
					when others => line_counter <= "00";
				end case;
				ppu_process_row <= std_logic_vector(unsigned(ppu_row) - 1);
			end if;
			if ppu_hstart_trigger = '1' and 
				(ppu_process_row = std_logic_vector(to_unsigned(237, 9)) or 
				ppu_process_row = std_logic_vector(to_unsigned(238, 9))) then
				ppu_last_row_count <= std_logic_vector(to_unsigned(258, 9)) & "0011";
			end if;
			if ppu_last_row_count(12 downto 4) /= "000000000" then
				case ppu_last_row_count(3 downto 0) is
					when "0000" => ppu_last_row_count(3 downto 0) <= "0001";
					when "0001" => ppu_last_row_count(3 downto 0) <= "0010";
					when "0010" => ppu_last_row_count(3 downto 0) <= "0011";
					when "0011" => ppu_last_row_count(3 downto 0) <= "0100";
					when "0100" => ppu_last_row_count(3 downto 0) <= "0101";
					when "0101" => ppu_last_row_count(3 downto 0) <= "0110";
					when "0110" => ppu_last_row_count(3 downto 0) <= "0111";
					when "0111" => ppu_last_row_count(3 downto 0) <= "1000";
					when "1000" => ppu_last_row_count(3 downto 0) <= "1001";
					when "1001" => ppu_last_row_count(3 downto 0) <= "1010";
					when "1010" => ppu_last_row_count(3 downto 0) <= "1011";
					when "1011" => ppu_last_row_count(3 downto 0) <= "1100";
					when others => 
						ppu_last_row_count(3 downto 0) <= "0000";
						ppu_last_row_count(12 downto 4) <= std_logic_vector(unsigned(ppu_last_row_count(12 downto 4)) - 1);
				end case;
			end if;
			if ppu_last_column_trigger or 
				(ppu_pixel_valid and ppu_pixel_trigger) or 
				(not ppu_pixel_valid and ppu_last_row_pixel_trigger) then
					ppu_last_column_count <= "1101";
			else
				case ppu_last_column_count is
					when "1101" => ppu_last_column_count <= "1100";
					when "1100" => ppu_last_column_count <= "1011";
					when "1011" => ppu_last_column_count <= "1010";
					when "1010" => ppu_last_column_count <= "1001";
					when "1001" => ppu_last_column_count <= "1000";
					when "1000" => ppu_last_column_count <= "0111";
					when "0111" => ppu_last_column_count <= "0110";
					when "0110" => ppu_last_column_count <= "0101";
					when "0101" => ppu_last_column_count <= "0100";
					when "0100" => ppu_last_column_count <= "0011";
					when "0011" => ppu_last_column_count <= "0010";
					when "0010" => ppu_last_column_count <= "0001";
					when others => ppu_last_column_count <= "0000";
				end case;
			end if;
			ppu_process_column <= std_logic_vector(unsigned(ppu_column) - 2);
			
			if ppu_pixel_trigger or ppu_last_column_trigger then
				ppu_subpixel <= "0001";
			else
				case ppu_subpixel is
					when "0001" => ppu_subpixel <= "0010";
					when "0010" => ppu_subpixel <= "0011";
					when "0011" => ppu_subpixel <= "0100";
					when "0100" => ppu_subpixel <= "0101";
					when "0101" => ppu_subpixel <= "0110";
					when "0110" => ppu_subpixel <= "0111";
					when "0111" => ppu_subpixel <= "1000";
					when "1000" => ppu_subpixel <= "1001";
					when "1001" => ppu_subpixel <= "1010";
					when "1010" => ppu_subpixel <= "1011";
					when "1011" => ppu_subpixel <= "1100";
					when others => ppu_subpixel <= "0000";
				end case;
			end if;
			if ppu_pixel_trigger then
				case line_counter is
					when "00" => 
						line0_address <= ppu_column(7 downto 0);
						line0_din <= ppu_pixel;
						line0_rw <= '0';
					when "01" => 
						line1_address <= ppu_column(7 downto 0);
						line1_din <= ppu_pixel;
						line1_rw <= '0';
					when others =>
						line2_address <= ppu_column(7 downto 0);
						line2_din <= ppu_pixel;
						line2_rw <= '0';
				end case;
			else
				line0_rw <= '1';
				line1_rw <= '1';
				line2_rw <= '1';
				case line_counter is
					when "00" => 
						line1_address <= ppu_column(7 downto 0);
						line2_address <= ppu_column(7 downto 0);
					when "01" => 
						line0_address <= ppu_column(7 downto 0);
						line2_address <= ppu_column(7 downto 0);
					when others =>
						line0_address <= ppu_column(7 downto 0);
						line1_address <= ppu_column(7 downto 0);
				end case;
			end if;
			
			if ppu_rescale_row and ppu_hstart_trigger then
				case line_out_counter is
					when 0 => line_out_counter <= 1;
					when 1 => line_out_counter <= 2;
					when 2 => line_out_counter <= 3;
					when 3 => line_out_counter <= 4;
					when 4 => line_out_counter <= 5;
					when others => line_out_counter <= 0;
				end case;
			end if;
			
			case ppu_subpixel_process is
				when "0001" =>
					kernel_a <= kernel_b;
					kernel_b <= kernel_c;
					if ppu_border(BORDER_UP) and ppu_border(BORDER_LEFT_RIGHT) then
						case line_counter is
							when "00" => 
								kernel_c <= line1_dout;
							when "01" => 
								kernel_c <= line2_dout;
							when others => 
								kernel_c <= line0_dout;
						end case;
					else
						kernel_c <= (others => '0');
					end if;
					kernel_d <= kernel_e;
					kernel_e <= kernel_f;
					if ppu_border(BORDER_LEFT_RIGHT) then
						case line_counter is
							when "00" =>
								kernel_f <= line2_dout;
							when "01" =>
								kernel_f <= line0_dout;
							when others =>
								kernel_f <= line1_dout;
						end case;
					else
						kernel_f <= (others => '0');
					end if;
					kernel_g <= kernel_h;
					kernel_h <= kernel_i;
					if ppu_border(BORDER_DOWN) and ppu_border(BORDER_LEFT_RIGHT) then
						case line_counter is
							when "00" => 
								kernel_i <= line0_dout;
							when "01" => 
								kernel_i <= line1_dout;
							when others => 
								kernel_i <= line2_dout;
						end case;
					else
						kernel_i <= (others => '0');
					end if;
				when "0010" =>
				when others =>
			end case;
			
			if hdmi_valid_calc = '1' and ppu_subpixel_process > std_logic_vector(to_unsigned(3, 4)) then
				if hdmi_column_calc /= 767 then
					hdmi_column_calc <= hdmi_column_calc + 1;
				else
					hdmi_column_calc <= 0;
					if hdmi_row_calc /= 5 then
						hdmi_row_calc <= hdmi_row_calc + 1;
					else
						hdmi_row_calc <= 0;
					end if;
				end if;
			end if;

			if ppu_subpixel_process > std_logic_vector(to_unsigned(1, 4)) then
				case hdmi_row_calc is
					when 0 => line_out_0_address <= std_logic_vector(to_unsigned(hdmi_column_calc, 10));
					when 1 => line_out_1_address <= std_logic_vector(to_unsigned(hdmi_column_calc, 10));
					when 2 => line_out_2_address <= std_logic_vector(to_unsigned(hdmi_column_calc, 10));
					when 3 => line_out_3_address <= std_logic_vector(to_unsigned(hdmi_column_calc, 10));
					when 4 => line_out_4_address <= std_logic_vector(to_unsigned(hdmi_column_calc, 10));
					when others => line_out_5_address <= std_logic_vector(to_unsigned(hdmi_column_calc, 10));
				end case;
			end if;
			
			if ppu_rescale_column = '1' and ppu_rescale_row = '1' then
				line_out_0_rw <= '1';
				line_out_1_rw <= '1';
				line_out_2_rw <= '1';
				line_out_3_rw <= '1';
				line_out_4_rw <= '1';
				line_out_5_rw <= '1';
				case ppu_subpixel_process is
					when "0001" =>
						if ppu_process_column /= std_logic_vector(to_unsigned(0, 9)) then
							ppu_rescale_out_column1 <= ppu_rescale_out_column1 + 3;
							ppu_rescale_out_column2 <= ppu_rescale_out_column2 + 3;
							ppu_rescale_out_column3 <= ppu_rescale_out_column3 + 3;
						end if;
					when "0010" =>
					when "0011" =>
						case line_out_counter is
							when 1 | 3 | 5 =>
								line_out_0_din <= kernel_out_a;
								line_out_0_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column1, 10));
								line_out_0_rw <= '0';
								line_out_1_din <= kernel_out_d;
								line_out_1_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column1, 10));
								line_out_1_rw <= '0';
								line_out_2_din <= kernel_out_g;
								line_out_2_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column1, 10));
								line_out_2_rw <= '0';
							when others =>
								line_out_3_din <= kernel_out_a;
								line_out_3_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column1, 10));
								line_out_3_rw <= '0';
								line_out_4_din <= kernel_out_d;
								line_out_4_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column1, 10));
								line_out_4_rw <= '0';
								line_out_5_din <= kernel_out_g;
								line_out_5_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column1, 10));
								line_out_5_rw <= '0';
						end case;
					when "0100" =>
						case line_out_counter is
							when 1 | 3 | 5 =>
								line_out_0_din <= kernel_out_b;
								line_out_0_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column2, 10));
								line_out_0_rw <= '0';
								line_out_1_din <= kernel_out_e;
								line_out_1_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column2, 10));
								line_out_1_rw <= '0';
								line_out_2_din <= kernel_out_h;
								line_out_2_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column2, 10));
								line_out_2_rw <= '0';
							when others =>
								line_out_3_din <= kernel_out_b;
								line_out_3_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column2, 10));
								line_out_3_rw <= '0';
								line_out_4_din <= kernel_out_e;
								line_out_4_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column2, 10));
								line_out_4_rw <= '0';
								line_out_5_din <= kernel_out_h;
								line_out_5_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column2, 10));
								line_out_5_rw <= '0';
						end case;
					when "0101" =>
						case line_out_counter is
							when 1 | 3 | 5 =>
								line_out_0_din <= kernel_out_c;
								line_out_0_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column3, 10));
								line_out_0_rw <= '0';
								line_out_1_din <= kernel_out_f;
								line_out_1_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column3, 10));
								line_out_1_rw <= '0';
								line_out_2_din <= kernel_out_i;
								line_out_2_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column3, 10));
								line_out_2_rw <= '0';
							when others =>
								line_out_3_din <= kernel_out_c;
								line_out_3_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column3, 10));
								line_out_3_rw <= '0';
								line_out_4_din <= kernel_out_f;
								line_out_4_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column3, 10));
								line_out_4_rw <= '0';
								line_out_5_din <= kernel_out_i;
								line_out_5_address <= std_logic_vector(to_unsigned(ppu_rescale_out_column3, 10));
								line_out_5_rw <= '0';
						end case;
					when "0110" =>
					when "0111" =>
					when "1000" =>
					when "1001" =>
					when "1010" =>
					when others =>
				end case;
			end if;
		end if;
	end process;
	
	frame_sync: entity work.frame_sync port map(
		clock => fast_clock,
		sync1 => ppu_vsync_sync,
		sync2 => hdmi_vsync_trigger,
		hsync1 => hdmi_line_done_sig,
		hsync2 => hdmi_line_ready,
		pause => fsync_pause);
	
	rescale_kernel: entity work.resize_kernel3 port map(
		din_a => kernel_a, din_b => kernel_b, din_c => kernel_c,
		din_d => kernel_d, din_e => kernel_e, din_f => kernel_f,
		din_g => kernel_g, din_h => kernel_h, din_i => kernel_i,
		dout_a => kernel_out_a, dout_b => kernel_out_b, dout_c => kernel_out_c,
		dout_d => kernel_out_d, dout_e => kernel_out_e, dout_f => kernel_out_f,
		dout_g => kernel_out_g, dout_h => kernel_out_h, dout_i => kernel_out_i,
		clock => fast_clock,
		trigger => ppu_rescale_trigger,
		mode => "0");
	
	line_out_0: entity work.clocked_sram
		generic map(bits => 10, dbits => 24)
		port map(clock => fast_clock,
			fast_clock => fast_clock,
			address => line_out_0_address,
			rw => line_out_0_rw,
			cs => '1',
			dout => line_out_0_dout,
			dout_valid => line_out_0_dout_valid,
			din => line_out_0_din);
	
	line_out_1: entity work.clocked_sram
		generic map(bits => 10, dbits => 24)
		port map(clock => fast_clock,
			fast_clock => fast_clock,
			address => line_out_1_address,
			rw => line_out_1_rw,
			cs => '1',
			dout => line_out_1_dout,
			dout_valid => line_out_1_dout_valid,
			din => line_out_1_din);
	
	line_out_2: entity work.clocked_sram
		generic map(bits => 10, dbits => 24)
		port map(clock => fast_clock,
			fast_clock => fast_clock,
			address => line_out_2_address,
			rw => line_out_2_rw,
			cs => '1',
			dout => line_out_2_dout,
			dout_valid => line_out_2_dout_valid,
			din => line_out_2_din);
	
	line_out_3: entity work.clocked_sram
		generic map(bits => 10, dbits => 24)
		port map(clock => fast_clock,
			fast_clock => fast_clock,
			address => line_out_3_address,
			rw => line_out_3_rw,
			cs => '1',
			dout => line_out_3_dout,
			dout_valid => line_out_3_dout_valid,
			din => line_out_3_din);
	
	line_out_4: entity work.clocked_sram
		generic map(bits => 10, dbits => 24)
		port map(clock => fast_clock,
			fast_clock => fast_clock,
			address => line_out_4_address,
			rw => line_out_4_rw,
			cs => '1',
			dout => line_out_4_dout,
			dout_valid => line_out_4_dout_valid,
			din => line_out_4_din);
			
	line_out_5: entity work.clocked_sram
		generic map(bits => 10, dbits => 24)
		port map(clock => fast_clock,
			fast_clock => fast_clock,
			address => line_out_5_address,
			rw => line_out_5_rw,
			cs => '1',
			dout => line_out_5_dout,
			dout_valid => line_out_5_dout_valid,
			din => line_out_5_din);
	
	line0: entity work.clocked_sram
		generic map(bits => 8, dbits => 24)
		port map(clock => fast_clock,
			fast_clock => fast_clock,
			address => line0_address,
			rw => line0_rw,
			cs => '1',
			dout => line0_dout,
			dout_valid => line0_dout_valid,
			din => line0_din);
	
	line1: entity work.clocked_sram
		generic map(bits => 8, dbits => 24)
		port map(clock => fast_clock,
			fast_clock => fast_clock,
			address => line1_address,
			rw => line1_rw,
			cs => '1',
			dout => line1_dout,
			dout_valid => line1_dout_valid,
			din => line1_din);
	
	line2: entity work.clocked_sram
		generic map(bits => 8, dbits => 24)
		port map(clock => fast_clock,
			fast_clock => fast_clock,
			address => line2_address,
			rw => line2_rw,
			cs => '1',
			dout => line2_dout,
			dout_valid => line2_dout_valid,
			din => line2_din);
	
	cpu: entity work.nes_cpu generic map(
		ramtype => ramtype) port map (
		pause_cpu => pause,
		d_a => d_a,
		d_x => d_x,
		d_y => d_y,
		d_pc => d_pc,
		d_sp => d_sp,
		d_flags => d_flags,
		d_subcycle => d_subcycle,
		d_cycle => d_cycle,
		instruction_toggle_out => instruction_toggle_out,
		clock => clock,
		ppu_clock => ppu_clock,
		memory_clock => cpu_memory_clock,
		memory_cycle_done => cpu_dready,
		rw => cpu_rw,
		oe => cpu_oe,
		reset => reset_sync,
		din => cpu_din,
		dout => cpu_dout,
		nmi => '1',
		irq => '1',
		tst => '0',
		address => cpu_address);
	
	ppu: entity work.nes_ppu generic map(
		random_noise => random_noise,
		ramtype => ramtype) port map (
		r_out => ppu_pixel(23 downto 16),
		g_out => ppu_pixel(15 downto 8),
		b_out => ppu_pixel(7 downto 0),
		pixel_valid => ppu_pixel_valid,
		hstart => ppu_hstart,
		vstart => ppu_vstart,
		row => ppu_row,
		column => ppu_column,
		clock => ppu_clock,
		reset => reset_sync,
		cpu_addr => cpu_address(2 downto 0),
		cpu_cs => cpu_ppu_cs,
		cpu_rw => cpu_rw,
		cpu_dout => cpu_dout,
		cpu_din => cpu_ppu_din,
		cpu_mem_clock => memory_clock,
		int => ppu_int,
		ppu_ale => ppu_ale,
		ppu_ad => ppu_ad,
		ppu_a => ppu_a,
		ppu_din => ppu_din,
		ppu_rd => ppu_rd,
		ppu_wr => ppu_wr);
	
	cartridge: entity work.nes_cartridge generic map(
		ramtype => ramtype,
		unified_ram => unified_ram) port map (
		cpu_data_out => cpu_dout,
		cpu_data_in => cpu_cartridge_din,
		cpu_data_in_ready => cpu_cartridge_din_ready,
		cpu_addr => cpu_address,
		ppu_data_in => "00000000",
		ppu_addr => "00000000000000",
		ppu_addr_a13_n => '1',
		ppu_wr => '0',
		ppu_rd => '0',
		cpu_rw => cpu_rw,
		romsel => cpu_address(15),
		m2 => memory_clock,
		clock => clock,
		write_signal => write_signal,
		write_address => write_address,
		write_value => write_value,
		write_trigger => write_trigger,
		write_rw => write_rw,
		write_cs => write_cs
	);

end Behavioral;

