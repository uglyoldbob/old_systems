library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity wishbone_host_combiner is
    generic (
        sim: in std_logic := '0');
    port (
        wba_ack: out std_logic;
		wba_d_miso: out std_logic_vector(31 downto 0);
		wba_d_mosi: in std_logic_vector(31 downto 0);
		wba_err: out std_logic;
		wba_addr: in std_logic_vector(29 downto 0);
		wba_bte: in std_logic_vector(1 downto 0);
		wba_cti: in std_logic_vector(2 downto 0);
		wba_cyc: in std_logic;
		wba_sel: in std_logic_vector(3 downto 0);
		wba_stb: in std_logic;
		wba_we: in std_logic;
        wbb_ack: out std_logic;
		wbb_d_miso: out std_logic_vector(31 downto 0);
		wbb_d_mosi: in std_logic_vector(31 downto 0);
		wbb_err: out std_logic;
		wbb_addr: in std_logic_vector(29 downto 0);
		wbb_bte: in std_logic_vector(1 downto 0);
		wbb_cti: in std_logic_vector(2 downto 0);
		wbb_cyc: in std_logic;
		wbb_sel: in std_logic_vector(3 downto 0);
		wbb_stb: in std_logic;
		wbb_we: in std_logic;
        wbo_ack: in std_logic;
		wbo_d_miso: in std_logic_vector(31 downto 0);
		wbo_d_mosi: out std_logic_vector(31 downto 0);
		wbo_err: in std_logic;
		wbo_addr: out std_logic_vector(29 downto 0);
		wbo_bte: out std_logic_vector(1 downto 0);
		wbo_cti: out std_logic_vector(2 downto 0);
		wbo_cyc: out std_logic;
		wbo_sel: out std_logic_vector(3 downto 0);
		wbo_stb: out std_logic;
		wbo_we: out std_logic;
        clock: in std_logic);
end wishbone_host_combiner;

architecture Behavioral of wishbone_host_combiner is
    signal mode: integer range 0 to 3 := 0;
begin

    process (all)
    begin
        case mode is
            when 1 => 
                wba_ack <= wbo_ack;
                wbb_ack <= '0';
                wba_d_miso <= wbo_d_miso;
                wbo_d_mosi <= wba_d_mosi;
                wbo_addr <= wba_addr;
                if sim then
                    wbb_d_miso <= (others => 'X');
                else
                    wbb_d_miso <= (others => '0');
                end if;
                wbo_bte <= wba_bte;
                wbo_cti <= wba_cti;
                wbo_cyc <= wba_cyc;
                wbo_sel <= wba_sel;
                wbo_stb <= wba_stb;
                wbo_we <= wba_we;
                wba_err <= wbo_err;
                wbb_err <= '0';
            when 2 =>
                wba_ack <= '0';
                wbb_ack <= wbo_ack;
                wbb_d_miso <= wbo_d_miso;
                wbo_d_mosi <= wbb_d_mosi;
                wbo_addr <= wbb_addr;
                if sim then
                    wba_d_miso <= (others => 'X');
                else
                    wba_d_miso <= (others => '0');
                end if;
                wbo_bte <= wbb_bte;
                wbo_cti <= wbb_cti;
                wbo_cyc <= wbb_cyc;
                wbo_sel <= wbb_sel;
                wbo_stb <= wbb_stb;
                wbo_we <= wbb_we;
                wbb_err <= wbo_err;
                wba_err <= '0';
            when others => 
                wba_ack <= '0';
                wbb_ack <= '0';
                if sim then
					wbo_addr <= (others => 'X');
                    wba_d_miso <= (others => 'X');
                    wbb_d_miso <= (others => 'X');
                    wbo_d_mosi <= (others => 'X');
                else
					wbo_addr <= (others => '0');
                    wba_d_miso <= (others => '0');
                    wbb_d_miso <= (others => '0');
                    wbo_d_mosi <= (others => '0');
                end if;
                wbo_bte <= "00";
                wbo_cti <= "000";
                wbo_cyc <= '0';
                wbo_sel <= "0000";
                wbo_stb <= '0';
                wbo_we <= '0';
                wba_err <= '0';
                wbb_err <= '0';
        end case;
    end process;

    process (clock)
    begin
        if rising_edge(clock) then
            case mode is
                when 0 =>
                    if wba_cyc then
                        mode <= 1;
                    elsif wbb_cyc then
                        mode <= 2;
                    end if;
                when 1 =>
                    if wba_ack or wba_err then
                        mode <= 0;
                    end if;
                when 2 =>
                    if wbb_ack or wbb_err then
                        mode <= 0;
                    end if;
                when others => null;
            end case;
        end if;
    end process;
end Behavioral;