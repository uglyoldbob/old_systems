library IEEE;
use IEEE.STD_LOGIC_1164.ALL;

entity ddr is
	Generic(t: string := "clock");
	Port( din: in std_logic_vector(1 downto 0);
			dout: out std_logic;
			clock: in std_logic);
end ddr;

architecture Behavioral of ddr is
begin
	process (all)
	begin
		if t = "clock" then
			if rising_edge(clock) then
				dout <= din(0);
			elsif falling_edge(clock) then
				dout <= din(1);
			end if;
		elsif t = "mux" then
			if clock then
				dout <= din(0);
			else
				dout <= din(1);
			end if;
		end if;
	end process;
end Behavioral;