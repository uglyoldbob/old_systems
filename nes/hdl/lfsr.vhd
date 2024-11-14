library ieee; 
use ieee.std_logic_1164.all;

entity lfsr32 is 
	port (
		clock: in std_logic;
		dout: out std_logic_vector(31 downto 0)
		);
end lfsr32;

architecture Behavioral of lfsr32 is  
	signal d: std_logic_vector(31 downto 0) := (0 => '1', others => '0');
	signal e: std_logic;
begin
	dout <= d;
	e <= d(31) xnor d(21) xnor d(1) xnor d(0);
	process (clock)
	begin
		if rising_edge(clock) then
			d <= d(30 downto 0) & e;
		end if;
	end process;
end Behavioral;