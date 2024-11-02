library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;
use std.textio.all;
use ieee.std_logic_textio.all;

entity clocked_sram is
	Generic (
		bits: integer := 11;
		filename: string := "none");
	Port (
		clock: in std_logic;
		cs: in std_logic;
		address: in std_logic_vector(bits-1 downto 0);
		rw: in std_logic;
		din: in std_logic_vector(7 downto 0);
		dout: out std_logic_vector(7 downto 0));
end clocked_sram;

architecture Behavioral of clocked_sram is
type RAM_ARRAY is array (2**bits-1 downto 0) of std_logic_vector (7 downto 0);

impure function InitRomFromFile (RomFileName : in string) return RAM_ARRAY is
	variable rom : RAM_ARRAY;
	FILE romfile : text is in RomFileName;
	variable open_status :FILE_OPEN_STATUS;
	file     infile      :text;
	variable RomFileLine : line;
	begin
		if RomFileName /= "none" then
			file_open(open_status, infile, filename, read_mode);
         if open_status = open_ok then
				for i in RAM_ARRAY'low to RAM_ARRAY'high loop
					readline(romfile, RomFileLine);
					hread(RomFileLine, rom(i));
				end loop;
			end if;
		end if;
		return rom;
	end function;
signal ram: RAM_ARRAY := InitRomFromFile(filename);
begin
	process (clock)
	begin
		if rising_edge(clock) then
			if cs = '1' then
				if rw = '0' then
					ram(to_integer(unsigned(address))) <= din;
				else
					dout <= ram(to_integer(unsigned(address)));
				end if;
			end if;
		end if;
	end process;
end Behavioral;
