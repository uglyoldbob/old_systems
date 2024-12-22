library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;
use std.textio.all;
use ieee.std_logic_textio.all;

entity clocked_sram_init is
	Generic (
		bits: integer := 11;
		dbits: integer := 8;
		filename: string := "none");
	Port (
		clock: in std_logic;
		cs: in std_logic;
		address: in std_logic_vector(bits-1 downto 0);
		rw: in std_logic;
		din: in std_logic_vector(dbits-1 downto 0);
		dout_valid: out std_logic;
		dout: out std_logic_vector(dbits-1 downto 0));
end clocked_sram_init;

architecture Behavioral of clocked_sram_init is
type RAM_ARRAY is array (2**bits-1 downto 0) of std_logic_vector (dbits-1 downto 0);

impure function InitRomFromFile (RomFileName : in string) return RAM_ARRAY is
	variable rom : RAM_ARRAY := (others => (others => '0'));
	FILE romfile : text is in RomFileName;
	variable open_status :FILE_OPEN_STATUS;
    variable rom_value: std_logic_vector(dbits-1 downto 0);
	variable i: integer;
	file     infile      :text;
	variable RomFileLine : line;
	begin
		if RomFileName /= "none" then
			file_open(open_status, infile, filename, read_mode);
         if open_status = open_ok then
				i := 0;
				while not endfile(romfile) and i < RAM_ARRAY'high loop
					readline(romfile, RomFileLine);
					hread(RomFileLine, rom_value);
                    if dbits=32 then
                        rom(i) := rom_value(7 downto 0) & rom_value(15 downto 8) & rom_value(23 downto 16) & rom_value(31 downto 24);
                    else
                        rom(i) := rom_value;
                    end if;
					i := i + 1;
				end loop;
			else
				report "Unable to open " & RomFileName severity error;
			end if;
		end if;
		return rom;
	end function;
signal ram: RAM_ARRAY := InitRomFromFile(filename);

signal out_addr: std_logic_vector(bits-1 downto 0);
begin
	process (all)
	begin
		if out_addr = address then
			dout_valid <= '1';
		else
			dout_valid <= '0';
		end if;
	end process;
	process (clock)
	begin
		if rising_edge(clock) then
			if cs = '1' then
				if rw = '0' then
					ram(to_integer(unsigned(address))) <= din;
				else
					dout <= ram(to_integer(unsigned(address)));
					out_addr <= address;
				end if;
			end if;
		end if;
	end process;
end Behavioral;
