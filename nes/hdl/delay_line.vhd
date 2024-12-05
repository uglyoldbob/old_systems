library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity delay_line is
	Port(
		clock: in std_logic;
		start: in std_logic;
		done: in std_logic;
		ready: out std_logic
		);
end delay_line;

architecture Behavioral of delay_line is
signal r1: std_logic;
begin
	process (clock, r1)
	begin
		ready <= r1;
		if rising_edge(clock) then
			if start then
				r1 <= '0';
			elsif done then
				r1 <= '1';
			end if;
		end if;
	end process;
end Behavioral;