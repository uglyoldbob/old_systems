library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity nes is
	Generic (
		ramtype: string := "sram";
		random_noise: in std_logic := '1';
		unified_ram: std_logic := '0');
   Port (
		write_signal: in std_logic := '0';
		write_address: in std_logic_vector(19 downto 0) := (others=>'0');
		write_value: in std_logic_vector(7 downto 0) := (others=>'0');
		write_trigger: in std_logic := '0';
		write_rw: in std_logic;
		write_cs: in std_logic_vector(1 downto 0) := (others=>'0');
		asdf: out std_logic;
		ppu_r: out std_logic_vector(7 downto 0);
		ppu_g: out std_logic_vector(7 downto 0);
		ppu_b: out std_logic_vector(7 downto 0);
		
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
	   clock: in std_logic;
		cpu_oe: out std_logic_vector(1 downto 0);
		cpu_memory_address: out std_logic_vector(15 downto 0);
	   whocares: out std_logic;
		cs_out: out std_logic_vector(3 downto 0);
		otherstuff: out std_logic_vector(15 downto 0));
end nes;

architecture Behavioral of nes is
	type PPU_LINE_BUFFER is array (255 downto 0) of std_logic_vector (23 downto 0);
	
	signal line0: PPU_LINE_BUFFER;
	signal line1: PPU_LINE_BUFFER;
	signal line2: PPU_LINE_BUFFER;
	
	signal kernel_a: std_logic_vector(23 downto 0);
	signal kernel_b: std_logic_vector(23 downto 0);
	signal kernel_c: std_logic_vector(23 downto 0);
	signal kernel_d: std_logic_vector(23 downto 0);
	signal kernel_e: std_logic_vector(23 downto 0);
	signal kernel_f: std_logic_vector(23 downto 0);
	signal kernel_g: std_logic_vector(23 downto 0);
	signal kernel_h: std_logic_vector(23 downto 0);
	signal kernel_i: std_logic_vector(23 downto 0);
	
	signal line_counter: std_logic_vector(1 downto 0) := (others => '0');

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
	signal ppu_row: std_logic_vector(7 downto 0);
	signal ppu_column: std_logic_vector(7 downto 0);
	signal ppu_subpixel: std_logic_vector(1 downto 0);
	signal ppu_subpixel_process: std_logic_vector(1 downto 0);
	
	signal ppu_last_column_trigger: std_logic;
	signal ppu_last_column_count: std_logic_vector(2 downto 0) := (others => '0');
	signal ppu_last_row_trigger: std_logic;
	signal ppu_last_row_count: std_logic_vector(10 downto 0) := (others => '0');
	signal ppu_process_column: std_logic_vector(7 downto 0) := (others => '0');
	signal ppu_process_row: std_logic_vector(7 downto 0) := (others => '0');
	signal ppu_last_row_pixel_trigger: std_logic;
	signal ppu_first_row_skip: std_logic := '0';
	signal ppu_first_column_skip: std_logic := '0';
	signal ppu_border: std_logic_vector(3 downto 0); --LURD (left, up, right, down)
	
	signal cpu_apu_cs: std_logic;
	
	signal cpu_cartridge_cs: std_logic;
	signal cpu_cartridge_din: std_logic_vector(7 downto 0);
	signal cpu_cartridge_din_ready: std_logic;

	signal pause: std_logic;
	
	signal reset_sync: std_logic;
	signal reset_chain: std_logic;
begin
	whocares <= clock;
	otherstuff <= cpu_address;
	cpu_memory_address <= cpu_address;
	cs_out <= cpu_ram_cs & cpu_ppu_cs & cpu_apu_cs & cpu_cartridge_cs;
	
	d_memory_clock <= memory_clock;
	pause <= write_signal or (cpu_memory_clock and not cpu_din_ready);
	
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
	ppu_vstart_trigger <= ppu_clock and ppu_vstart_delay;
	process (all)
	begin
		if ppu_last_column_count = "001" then
			ppu_last_column_trigger <= '1';
		else
			ppu_last_column_trigger <= '0';
		end if;
		if not ppu_last_row_trigger then
			ppu_subpixel_process <= ppu_subpixel;
		else
			ppu_subpixel_process <= ppu_last_row_count(1 downto 0);
		end if;
		if ppu_process_column > std_logic_vector(to_unsigned(0, 8)) then
			ppu_border(0) <= '1';
		else
			ppu_border(0) <= '0';
		end if;
		if ppu_process_column < std_logic_vector(to_unsigned(255, 8)) then
			ppu_border(2) <= '1';
		else
			ppu_border(2) <= '0';
		end if;
		if ppu_process_row > std_logic_vector(to_unsigned(0, 8)) then
			ppu_border(1) <= '1';
		else
			ppu_border(1) <= '0';
		end if;
		if ppu_process_row < std_logic_vector(to_unsigned(239, 8)) then
			ppu_border(3) <= '1';
		else
			ppu_border(3) <= '0';
		end if;
	end process;
	
	process (clock)
	begin
		if rising_edge(clock) then
			if ppu_vstart_trigger then
				ppu_first_row_skip <= '0';
			elsif ppu_row = std_logic_vector(to_unsigned(1, 8)) then
				ppu_first_row_skip <= '1';
			end if;
			if ppu_hstart_trigger then
				ppu_first_column_skip <= '0';
			elsif ppu_subpixel = "11" then
				ppu_first_column_skip <= '1';
			end if;
			ppu_hstart_trigger <= ppu_clock and ppu_hstart_delay;
			ppu_clock_delay <= ppu_clock;
			ppu_hstart_delay <= ppu_hstart and ppu_clock;
			ppu_vstart_delay <= ppu_vstart and ppu_clock;
			if ppu_last_row_count(10 downto 2) /= "000000000" then
				if ppu_last_row_count(1 downto 0) = "00" then
					ppu_last_row_pixel_trigger <= '1';
				else
					ppu_last_row_pixel_trigger <= '0';
				end if;
			else
				ppu_last_row_pixel_trigger <= '0';
			end if;
			if ppu_hstart_trigger then
				case line_counter is
					when "00" => line_counter <= "01";
					when "01" => line_counter <= "10";
					when others => line_counter <= "00";
				end case;
				ppu_process_row <= std_logic_vector(unsigned(ppu_row) - 1);
				if ppu_process_row = std_logic_vector(to_unsigned(238, 8)) then
					ppu_last_row_count <= std_logic_vector(to_unsigned(257, 9)) & "00";
				end if;
			end if;
			if ppu_last_row_count(10 downto 2) /= "000000000" then
				ppu_last_row_trigger <= '1';
				case ppu_last_row_count(1 downto 0) is
					when "00" => ppu_last_row_count(1 downto 0) <= "01";
					when "01" => ppu_last_row_count(1 downto 0) <= "10";
					when "10" => ppu_last_row_count(1 downto 0) <= "11";
					when others => 
						ppu_last_row_count(1 downto 0) <= "00";
						ppu_last_row_count(10 downto 2) <= std_logic_vector(unsigned(ppu_last_row_count(10 downto 2)) - 1);
				end case;
			else
				ppu_last_row_trigger <= '0';
			end if;
			if (ppu_last_row_pixel_trigger or ppu_last_column_trigger or ppu_pixel_trigger) and ppu_pixel_valid then
					ppu_last_column_count <= "101";
			else
				case ppu_last_column_count is
					when "101" => ppu_last_column_count <= "100";
					when "100" => ppu_last_column_count <= "011";
					when "011" => ppu_last_column_count <= "010";
					when "010" => ppu_last_column_count <= "001";
					when others => ppu_last_column_count <= "000";
				end case;
			end if;
			if ppu_pixel_trigger or ppu_last_row_trigger then
				ppu_process_column <= std_logic_vector(unsigned(ppu_column) - 1);
			end if;
			if ppu_pixel_trigger then
				ppu_subpixel <= "01";
				case line_counter is
					when "00" => line0(to_integer(unsigned(ppu_column))) <= ppu_r & ppu_g & ppu_b;
					when "01" => line1(to_integer(unsigned(ppu_column))) <= ppu_r & ppu_g & ppu_b;
					when others => line2(to_integer(unsigned(ppu_column))) <= ppu_r & ppu_g & ppu_b;
				end case;
			else
				case ppu_subpixel is
					when "01" => ppu_subpixel <= "10";
					when "10" => ppu_subpixel <= "11";
					when others => ppu_subpixel <= "00";
				end case;
			end if;

			if ppu_first_row_skip and (ppu_first_column_skip or ppu_last_row_trigger) then
				case ppu_subpixel_process is
					when "01" =>
						if ppu_border(0) and ppu_border(1) then
							case line_counter is
								when "00" =>
									kernel_a <= line1(to_integer(unsigned(ppu_process_column)-1));
								when "01" =>
									kernel_a <= line2(to_integer(unsigned(ppu_process_column)-1));
								when others =>
									kernel_a <= line0(to_integer(unsigned(ppu_process_column)-1));
							end case;
						else
							kernel_a <= (others => '0');
						end if;
						if ppu_border(1) then
							case line_counter is
								when "00" => 
									kernel_b <= line1(to_integer(unsigned(ppu_process_column)));
								when "01" => 
									kernel_b <= line2(to_integer(unsigned(ppu_process_column)));
								when others => 
									kernel_b <= line0(to_integer(unsigned(ppu_process_column)));
							end case;
						else
							kernel_b <= (others => '0');
						end if;
						if ppu_border(1) and ppu_border(2) then
							case line_counter is
								when "00" => 
									kernel_c <= line1(to_integer(unsigned(ppu_process_column)+1));
								when "01" => 
									kernel_c <= line2(to_integer(unsigned(ppu_process_column)+1));
								when others => 
									kernel_c <= line0(to_integer(unsigned(ppu_process_column)+1));
							end case;
						else
							kernel_c <= (others => '0');
						end if;
						if ppu_border(0) then
							case line_counter is
								when "00" =>
									kernel_d <= line2(to_integer(unsigned(ppu_process_column)-1));
								when "01" =>
									kernel_d <= line0(to_integer(unsigned(ppu_process_column)-1));
								when others =>
									kernel_d <= line1(to_integer(unsigned(ppu_process_column)-1));
							end case;
						else
							kernel_d <= (others => '0');
						end if;
						case line_counter is
							when "00" =>
								kernel_e <= line2(to_integer(unsigned(ppu_process_column)));
							when "01" =>
								kernel_e <= line0(to_integer(unsigned(ppu_process_column)));
							when others =>
								kernel_e <= line1(to_integer(unsigned(ppu_process_column)));
						end case;
						if ppu_border(2) then
							case line_counter is
								when "00" =>
									kernel_f <= line2(to_integer(unsigned(ppu_process_column)+1));
								when "01" =>
									kernel_f <= line0(to_integer(unsigned(ppu_process_column)+1));
								when others =>
									kernel_f <= line1(to_integer(unsigned(ppu_process_column)+1));
							end case;
						else
							kernel_f <= (others => '0');
						end if;
						if ppu_border(0) and ppu_border(3) then
							case line_counter is
								when "00" =>
									kernel_g <= line0(to_integer(unsigned(ppu_process_column)-1));
								when "01" =>
									kernel_g <= line1(to_integer(unsigned(ppu_process_column)-1));
								when others =>
									kernel_g <= line2(to_integer(unsigned(ppu_process_column)-1));
							end case;
						else
							kernel_g <= (others => '0');
						end if;
						if ppu_border(3) then
							case line_counter is
								when "00" => 
									kernel_h <= line0(to_integer(unsigned(ppu_process_column)));
								when "01" => 
									kernel_h <= line1(to_integer(unsigned(ppu_process_column)));
								when others => 
									kernel_h <= line2(to_integer(unsigned(ppu_process_column)));
							end case;
						else
							kernel_h <= (others => '0');
						end if;
						if ppu_border(3) and ppu_border(2) then
							case line_counter is
								when "00" => 
									kernel_i <= line0(to_integer(unsigned(ppu_process_column)+1));
								when "01" => 
									kernel_i <= line1(to_integer(unsigned(ppu_process_column)+1));
								when others => 
									kernel_i <= line2(to_integer(unsigned(ppu_process_column)+1));
							end case;
						else
							kernel_i <= (others => '0');
						end if;
					when "10" =>
					when "11" =>
					when others =>
				end case;
			end if;
		end if;
	end process;
	
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
		r_out => ppu_r,
		g_out => ppu_g,
		b_out => ppu_b,
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

