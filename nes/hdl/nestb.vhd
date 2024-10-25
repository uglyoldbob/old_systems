library IEEE;
use IEEE.STD_LOGIC_1164.ALL;

entity nestb is
    Port (whocares: out std_logic;
	   cpu_oe: out std_logic_vector(1 downto 0);
		cpu_memory_address: out std_logic_vector(15 downto 0);
		cs_out: out std_logic_vector(3 downto 0);
		otherstuff: out std_logic_vector(15 downto 0));
end nestb;

architecture Behavioral of nestb is
	signal cpu_clock: std_logic := '0';
	signal cpu_reset: std_logic := '0';
	signal cpu_address: std_logic_vector(15 downto 0);
	signal cpu_dout: std_logic_vector(7 downto 0);
	signal cpu_din: std_logic_vector(7 downto 0);
begin
	cpu_reset <= '1', '0' after 100ns;
	cpu_clock <= NOT cpu_clock after 10ns;
	otherstuff <= cpu_address;
	cpu_memory_address <= cpu_address;
	nes: entity work.nes port map (
		reset => cpu_reset,
		cpu_oe => cpu_oe,
		cpu_memory_address => cpu_address,
		cs_out => cs_out,
		whocares => whocares,
		clock => cpu_clock
		);

end Behavioral;

