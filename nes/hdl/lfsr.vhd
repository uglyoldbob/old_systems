library ieee; 
use ieee.std_logic_1164.all;

entity lfsr is 
	generic(
		BITS             : integer           := 32          ;
		POLY          : std_logic_vector  := "1100000") ;
	port (
		clock: in std_logic
		);
end lfsr;

architecture Behavioral of lfsr is  
	
begin  
	process (clock)
	begin
		if rising_edge(clock) then
			
		end if;
	end process;
end Behavioral;