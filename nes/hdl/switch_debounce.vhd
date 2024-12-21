library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity switch_debounce is
	Port(
		slowclock: in std_logic;
		clock: in std_logic;
		din: in std_logic;
		dout: out std_logic);
end switch_debounce;

architecture Behavioral of switch_debounce is
	signal din2: std_logic;
	signal din3: std_logic;
	signal din4: std_logic;
	signal din5: std_logic;
	signal din6: std_logic;

	signal din_combined: std_logic_vector(2 downto 0);
	signal prev_debounce: std_logic;
	signal debounce: std_logic;

    signal slow_rising: std_logic;
begin
	din_combined <= din6 & din5 & din4;

    trigger: entity work.edge_detect port map(
		clock => clock,
		sig => slow_clock,
		rising => slow_rising);

	process (clock)
	begin
		if rising_edge(clock) then
			din2 <= din;
			din3 <= din2;
			din4 <= din3;
			din5 <= din4;
			din6 <= din5;
			prev_debounce <= debounce;
			if debounce and not prev_debounce then
				dout <= '1';
			else
				dout <= '0';
			end if;
            if slow_rising then
                case din_combined is
                    when "000" => debounce <= '0';
                    when "100" => debounce <= '0';
                    when "111" => debounce <= '1';
                    when "011" => debounce <= '1';
                    when others => null;
                end case;
            end if;
		end if;
	end process;
end Behavioral;