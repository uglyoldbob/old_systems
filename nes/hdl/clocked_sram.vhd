library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;
use std.textio.all;
use ieee.std_logic_textio.all;

entity clocked_sram is
	Generic (
		bits: integer := 11);
	Port (
		clock: in std_logic;
		cs: in std_logic;
		address: in std_logic_vector(bits-1 downto 0);
		rw: in std_logic;
		din: in std_logic_vector(7 downto 0);
		dout: out std_logic_vector(7 downto 0)
		);
end clocked_sram;

architecture Behavioral of clocked_sram is
type RAM_ARRAY is array (2**bits-1 downto 0) of std_logic_vector (7 downto 0);
signal ram: RAM_ARRAY;
begin
	process (clock)
	begin
		if rising_edge(clock) then
			if cs then
				if not rw then
					ram(to_integer(unsigned(address))) <= din;
				else
					dout <= ram(to_integer(unsigned(address)));
				end if;
			end if;
		end if;
	end process;
end Behavioral;
