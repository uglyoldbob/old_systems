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

	signal write_signal: std_logic;
	signal write_address: std_logic_vector(19 downto 0);
	signal write_value: std_logic_vector(7 downto 0);
	signal write_trigger: std_logic;
	signal write_rw: std_logic;
	signal write_cs: std_logic_vector(1 downto 0);
	
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
	
	signal cpu_memory_clock: std_logic;
	signal cpu_instruction: std_logic;
	signal instruction_check: std_logic;
	
	type RAM_ARRAY is array (2**19 downto 0) of std_logic_vector (7 downto 0);
	signal rom : RAM_ARRAY;
	FILE romfile : text;
begin
	cpu_clock <= NOT cpu_clock after 10ns;
	otherstuff <= cpu_address;
	cpu_memory_address <= cpu_address;
	nes: entity work.nes port map (
		write_signal => write_signal,
		write_address => write_address,
		write_value => write_value,
		write_trigger => write_trigger,
		write_rw => write_rw,
		write_cs => write_cs,
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
		clock => cpu_clock
		);

	process
		variable RomFileLine : line;
		variable rom_value: std_logic_vector(7 downto 0);
		variable i: integer;
		variable size: integer;
	begin
		cpu_reset <= '1';
		write_rw <= '1';
		write_signal <= '0';
		write_trigger <= '0';
		file_open(romfile, "nestest_prg_rom.txt", read_mode);
		i := 0;
		while not endfile(romfile) loop
			readline(romfile, RomFileLine);
			hread(RomFileLine, rom_value);
			rom(i) <= rom_value;
			i := i + 1;
		end loop;
		report("Read " & to_string(i) & " bytes");
		write_signal <= '1';
		size := i;
		i := 0;
		write_cs <= "00";
		wait until rising_edge(cpu_clock);
		write_rw <= '0';
		for i in 0 to size-1 loop
			write_address <= std_logic_vector(to_unsigned(i, 20));
			write_value <= rom(i);
			write_trigger <= '1';
			wait until rising_edge(cpu_clock);
			write_trigger <= '0';
			wait until rising_edge(cpu_clock);
		end loop;
		--provide a start address mod for nestest rom
		write_address <= x"03ffc";
		write_value <= x"00";
		write_trigger <= '1';
		wait until rising_edge(cpu_clock);
		write_trigger <= '0';
		wait until rising_edge(cpu_clock);
		
		write_signal <= '0';
		write_rw <= '1';
		wait until rising_edge(cpu_clock);
		
		cpu_reset <= '0' after 100ns;
		instruction_check <= '0';
		wait until cpu_reset = '0';
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
		report "Test complete";
		assert false report "Not a failure, test complete" severity failure;
	end process;
end Behavioral;

