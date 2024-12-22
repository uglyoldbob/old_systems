library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity uart is
   Generic (
		FREQ : integer;
		BAUD : integer := 115200);
   Port (
        wb_ack: out std_logic;
		wb_d_miso: out std_logic_vector(31 downto 0);
		wb_d_mosi: in std_logic_vector(31 downto 0);
		wb_err: out std_logic;
		wb_addr: in std_logic_vector(3 downto 0);
		wb_bte: in std_logic_vector(1 downto 0);
		wb_cti: in std_logic_vector(2 downto 0);
		wb_cyc: in std_logic;
		wb_sel: in std_logic_vector(3 downto 0);
		wb_stb: in std_logic;
		wb_we: in std_logic;
		test: out std_logic;
		clock: in std_logic;
		tx: out std_logic;
		rx: in std_logic);
end uart;

architecture Behavioral of uart is
	signal CYCLE_AMOUNT : integer range 0 to FREQ / BAUD := (FREQ / BAUD);

	signal uart_clock: std_logic := '0';
	signal clock_divider: integer range 0 to FREQ / BAUD -1 := FREQ / BAUD -1;

	signal uart_bit_num: integer range 0 to 9 := 0;
	signal dout: std_logic_vector(9 downto 0);
	signal dout_ready: std_logic := '1';

	signal tx_out: std_logic;

    constant MODE_UNINIT : integer range 0 to 3 := 0;
    constant MODE_IDLE : integer range 0 to 3 := 1;
    constant MODE_RW : integer range 0 to 3 := 2;
    constant MODE_RW_WAIT : integer range 0 to 3 := 3;
    signal mode: integer range 0 to 3 := MODE_UNINIT;
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

    process (clock)
    begin
        if rising_edge(clock) then
            case mode is
                when MODE_UNINIT =>
                    mode <= MODE_IDLE;
                when MODE_IDLE =>
                    if wb_cyc then
                        mode <= MODE_RW;
                    end if;
                when MODE_RW =>
                    mode <= MODE_RW_WAIT;
                when MODE_RW_WAIT =>
                    null;
                when others => null;
            end case;
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

