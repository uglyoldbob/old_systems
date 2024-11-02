library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;
use std.textio.all;
use ieee.std_logic_textio.all;

entity sram_inout is
	Generic (
		bits: integer := 11;
		filename: string := "none");
	Port (
		cs: in std_logic;
		address: in std_logic_vector(bits-1 downto 0);
		rw: in std_logic;
		din: in std_logic_vector(7 downto 0);
		dout: out std_logic_vector(7 downto 0));
end sram_inout;

architecture Behavioral of sram_inout is
type RAM_ARRAY is array (2**bits-1 downto 0) of std_logic_vector (7 downto 0);
signal ram: RAM_ARRAY;
begin
	process (all)
	begin
		if cs = '1' then
			if rw = '0' then
				ram(to_integer(unsigned(address))) <= din;
				dout <= (others => '0');
			else
				dout <= ram(to_integer(unsigned(address)));
			end if;
		else
			dout <= (others => '0');
		end if;
	end process;
end Behavioral;


library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;
use std.textio.all;
use ieee.std_logic_textio.all;

entity sram_inout_init is
	Generic (
		bits: integer := 11;
		filename: string := "none");
	Port (
		cs: in std_logic;
		address: in std_logic_vector(bits-1 downto 0);
		rw: in std_logic;
		din: in std_logic_vector(7 downto 0);
		dout: out std_logic_vector(7 downto 0));
end sram_inout_init;

architecture Behavioral of sram_inout_init is
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
	process (all)
	begin
		if cs = '1' then
			if rw = '0' then
				ram(to_integer(unsigned(address))) <= din;
				dout <= (others => '0');
			else
				dout <= ram(to_integer(unsigned(address)));
			end if;
		else
			dout <= (others => '0');
		end if;
	end process;
end Behavioral;
