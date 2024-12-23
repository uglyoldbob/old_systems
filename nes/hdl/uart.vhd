library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity uart is
   Generic (
        TX_DEPTH: integer := 4;
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

    --bit 10 is data_valid
    type TX_FIFO is array(TX_DEPTH-1 downto 0) of std_logic_vector(10 downto 0);

    signal fifo_tx: TX_FIFO := (others => (others => '0'));

	signal uart_clock: std_logic := '0';
	signal clock_divider: integer range 0 to FREQ / BAUD -1 := FREQ / BAUD -1;

	signal uart_bit_num: integer range 0 to 9 := 0;
	signal dout: std_logic_vector(9 downto 0) := "1" & "01000001" & "0";
	signal dout_ready: std_logic := '1';

	signal tx_out: std_logic;

    signal settings1: std_logic_vector(31 downto 0) := (others => '0');
    signal settings2: std_logic_vector(31 downto 0) := (others => '0');
    signal settings3: std_logic_vector(31 downto 0) := (others => '0');
    signal settings4: std_logic_vector(31 downto 0) := (others => '0');

    constant ADDR_SETTINGS1: std_logic_vector(3 downto 0) := "0000";
    constant ADDR_SETTINGS2: std_logic_vector(3 downto 0) := "0001";
    constant ADDR_SETTINGS3: std_logic_vector(3 downto 0) := "0010";
    constant ADDR_SETTINGS4: std_logic_vector(3 downto 0) := "0011";
    constant ADDR_TX: std_logic_vector(3 downto 0) := "1000";

    constant MODE_UNINIT : integer range 0 to 7 := 0;
    constant MODE_IDLE : integer range 0 to 7 := 1;
    constant MODE_RW : integer range 0 to 7 := 2;
    constant MODE_RW_WAIT : integer range 0 to 7 := 3;
    constant MODE_RW_DONE: integer range 0 to 7 := 4;
    signal mode: integer range 0 to 7 := MODE_UNINIT;

	signal need_data: std_logic;

	signal address: std_logic_vector(6 downto 0);
begin
	address <= wb_addr & "000";

	process (all)
	begin
		if uart_bit_num = 9 and clock_divider = 0 then
			need_data <= '1';
		else
			need_data <= '0';
		end if;
	end process;

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
				end if;
			end if;
            if need_data then
                if fifo_tx(0)(10) then
                    dout <= fifo_tx(0)(9 downto 0);
                    for i in 0 to TX_DEPTH-2 loop
                        fifo_tx(i) <= fifo_tx(i+1);
                    end loop;
                    fifo_tx(TX_DEPTH-1) <= (others => '0');
                else
                    dout <= (others => '1');
                end if;
			else
				case mode is
					when MODE_UNINIT =>
						mode <= MODE_IDLE;
					when MODE_IDLE =>
						if wb_cyc then
							mode <= MODE_RW;
						end if;
					when MODE_RW =>
						mode <= MODE_RW_WAIT;
						if wb_we then
							case wb_addr is
								when ADDR_SETTINGS1 =>
									settings1 <= wb_d_mosi;
									mode <= MODE_RW_DONE;
								when ADDR_SETTINGS2 =>
									settings2 <= wb_d_mosi;
									mode <= MODE_RW_DONE;
								when ADDR_SETTINGS3 =>
									settings3 <= wb_d_mosi;
									mode <= MODE_RW_DONE;
								when ADDR_SETTINGS4 =>
									settings4 <= wb_d_mosi;
									mode <= MODE_RW_DONE;
								when ADDR_TX =>
									mode <= MODE_RW_WAIT;
								when others => null;
							end case;
						end if;
					when MODE_RW_WAIT =>
						case wb_addr is
							when ADDR_TX =>
								case TX_DEPTH is
									when 4 =>
										if fifo_tx(0)(10) = '0' then
											fifo_tx(0) <= "11" & wb_d_mosi(7 downto 0) & "0";
											mode <= MODE_RW_DONE;
										elsif fifo_tx(1)(10) = '0' then
											fifo_tx(1) <= "11" & wb_d_mosi(7 downto 0) & "0";
											mode <= MODE_RW_DONE;
										elsif fifo_tx(2)(10) = '0' then
											fifo_tx(2) <= "11" & wb_d_mosi(7 downto 0) & "0";
											mode <= MODE_RW_DONE;
										elsif fifo_tx(3)(10) = '0' then
											fifo_tx(3) <= "11" & wb_d_mosi(7 downto 0) & "0";
											mode <= MODE_RW_DONE;
										end if;
									when others => null;
								end case;
							when ADDR_SETTINGS1 | ADDR_SETTINGS2 | ADDR_SETTINGS3 | ADDR_SETTINGS4 =>
								mode <= MODE_RW_DONE;
							when others => null;
						end case;
					when MODE_RW_DONE =>
						mode <= MODE_IDLE;
					when others => null;
				end case;
			end if;
        end if;
    end process;

	test <= tx_out;
	tx <= tx_out;

	process (all)
	begin
        if mode = MODE_RW_DONE then
            wb_ack <= '1';
        else
            wb_ack <= '0';
        end if;
		if dout_ready then
			tx_out <= dout(uart_bit_num);
		else
			tx_out <= '1';
		end if;
	end process;
end Behavioral;

