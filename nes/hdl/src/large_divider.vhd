library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity large_divider is
	Generic (
		bits: integer := 20);
   Port (
		clock: in std_logic;
        ckout: out std_logic);
end large_divider;

architecture Behavioral of large_divider is
	signal counter: std_logic_vector(bits-1 downto 0);
begin
	process (clock)
	begin
		if rising_edge(clock) then
			counter <= std_logic_vector(unsigned(counter) + 1);
			ckout <= counter(bits-1);
		end if;
	end process;
end Behavioral;

