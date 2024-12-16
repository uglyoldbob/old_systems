library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity uart is
   Generic (
		FREQ : integer;
		BAUD : integer := 115200);
   Port (
		test: out std_logic;
		clock: in std_logic;
		tx: out std_logic;
		rx: in std_logic);
end uart;

architecture Behavioral of uart is
	constant CYCLE_AMOUNT : integer := (FREQ / BAUD);

	signal uart_clock: std_logic := '0';
	signal clock_divider: integer range 0 to CYCLE_AMOUNT-1 := CYCLE_AMOUNT-1;

	signal uart_bit_num: integer range 0 to 9 := 0;
	signal dout: std_logic_vector(9 downto 0);
	signal dout_ready: std_logic := '1';

	signal tx_out: std_logic;
begin
	dout <= "1" & x"55" & "0";

	process (clock)
	begin
		if rising_edge(clock) then
			if clock_divider /= 0 then
				clock_divider <= clock_divider - 1;
			else
				clock_divider <= CYCLE_AMOUNT-1;
				uart_clock <= not uart_clock;
				if uart_bit_num /= 9 then
					uart_bit_num <= uart_bit_num + 1;
				else
					uart_bit_num <= 0;
					dout_ready <= not dout_ready;
				end if;
			end if;
		end if;
	end process;

	test <= tx_out;
	tx <= tx_out;

	process (all)
	begin
		if dout_ready then
			tx_out <= dout(uart_bit_num);
		else
			tx_out <= '1';
		end if;
	end process;
end Behavioral;

