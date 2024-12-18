library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity edge_detect is
	port (
		clock: in std_logic;
		sig: in std_logic;
		rising: out std_logic;
		falling: out std_logic);
end edge_detect;

architecture Behavioral of edge_detect is
	signal delay_sig: std_logic;
begin

	process(clock)
	begin
		if rising_edge(clock) then
			delay_sig <= sig;
			if sig and not delay_sig then
				rising <= '1';
			else
				rising <= '0';
			end if;
			if not sig and delay_sig then
				falling <= '1';
			else
				falling <= '0';
			end if;
		end if;
	end process;

end Behavioral;