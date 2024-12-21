library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;
use std.textio.all;
use ieee.std_logic_textio.all;

entity font_lookup_8x8 is
    generic (
        table_bits: integer := 7;
        table_name: string := "font.txt");
    port (
        clock: in std_logic;
        lookup_val: in std_logic_vector(table_bits-1 downto 0);
        row: in std_logic_vector(2 downto 0);
        column: in std_logic_vector(2 downto 0);
        visible: out std_logic);
end font_lookup_8x8;

architecture Behavioral of font_lookup_8x8 is
    --1 bit per pixel, 8 pixels per row, 8 rows per character, 128 characters total
    type ROM_DATA is array ((2**table_bits)*8-1 downto 0) of std_logic_vector (7 downto 0);

impure function LoadTable (FileName : in string) return ROM_DATA is
	variable rom : ROM_DATA := (others => (0 => '1', others => '0'));
	FILE romfile : text is in FileName;
	variable open_status :FILE_OPEN_STATUS;
	file     infile      :text;
	variable FileLine : line;
	begin
		if FileName /= "none" then
			file_open(open_status, infile, filename, read_mode);
         if open_status = open_ok then
				for i in ROM_DATA'low to ROM_DATA'high loop
					readline(romfile, FileLine);
					hread(FileLine, rom(i));
				end loop;
			end if;
		end if;
		return rom;
	end function;

    signal table: ROM_DATA := LoadTable(table_name);
    signal table_index: std_logic_vector(table_bits+2 downto 0);

    signal current_row: std_logic_vector(7 downto 0);
begin
    table_index <= lookup_val & row;
    current_row <= table(to_integer(unsigned(table_index)));
    process (all)
    begin
        case column is
            when "000" => visible <= current_row(0);
            when "001" => visible <= current_row(1);
            when "010" => visible <= current_row(2);
            when "011" => visible <= current_row(3);
            when "100" => visible <= current_row(4);
            when "101" => visible <= current_row(5);
            when "110" => visible <= current_row(6);
            when others => visible <= current_row(7);
        end case;
    end process;
end Behavioral;