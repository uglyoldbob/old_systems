library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;
use ieee.math_real.all;

entity gowin_sdram_interface is
	Generic(
        timing_cl: integer := 2;
        timing_al: integer := 2;
        clock_freq: positive;
        rambits: integer := 3);
    Port (
        reset: in std_logic;
        clock: in std_logic;
		O_sdram_clk: out std_logic;
		O_sdram_cke: out std_logic;
		O_sdram_cs_n: out std_logic;
		O_sdram_cas_n: out std_logic;
		O_sdram_ras_n: out std_logic;
		O_sdram_wen_n: out std_logic;
		O_sdram_dqm: out std_logic_vector(3 downto 0);
		O_sdram_addr: out std_logic_vector(10 downto 0);
		O_sdram_ba: out std_logic_vector(1 downto 0);
		IO_sdram_dq: inout std_logic_vector(31 downto 0);
		wb_ack: out std_logic;
		wb_d_miso: out std_logic_vector(2**rambits-1 downto 0);
		wb_d_mosi: in std_logic_vector(2**rambits-1 downto 0);
		wb_err: out std_logic;
		wb_addr: in std_logic_vector(25-rambits downto 0);
		wb_bte: in std_logic_vector(1 downto 0);
		wb_cti: in std_logic_vector(2 downto 0);
		wb_cyc: in std_logic;
		wb_sel: in std_logic_vector(rambits-3 downto 0);
		wb_stb: in std_logic;
		wb_we: in std_logic);
end gowin_sdram_interface;

architecture Behavioral of gowin_sdram_interface is

    constant REFI_COUNT : integer := clock_freq / 256000;

    signal refresh_needed: std_logic := '0';
    signal refresh_counter: integer range 0 to REFI_COUNT-1 := REFI_COUNT-1;
    signal refresh_row: std_logic_vector(11 downto 0) := (others => '0');

    signal delay_counter: integer range 0 to 15 := 0;

	signal I_sdrc_rst_n: std_logic;
	signal I_sdrc_clk: std_logic;
	signal I_sdram_clk: std_logic;
	signal I_sdrc_cmd_en: std_logic;
	signal I_sdrc_cmd: std_logic_vector(2 downto 0);
	signal I_sdrc_precharge_ctrl: std_logic;
	signal I_sdram_power_down: std_logic;
	signal I_sdram_selfrefresh: std_logic;
	signal I_sdrc_addr: std_logic_vector(20 downto 0);
	signal I_sdrc_dqm: std_logic_vector(3 downto 0);
	signal I_sdrc_data: std_logic_vector(31 downto 0);
	signal I_sdrc_data_len: std_logic_vector(7 downto 0);
	signal O_sdrc_data: std_logic_vector(31 downto 0);
	signal O_sdrc_init_done: std_logic;
	signal O_sdrc_cmd_ack: std_logic;

    constant MODE_UNINIT : integer range 0 to 3 := 0;
    constant MODE_IDLE : integer range 0 to 3 := 1;
    constant MODE_RW : integer range 0 to 3 := 2;
    constant MODE_RW_WAIT : integer range 0 to 3 := 3;
    signal mode: integer range 0 to 3 := MODE_UNINIT;

    constant SDRAM_MAX: integer := 15;
    constant SDRAM_UNINIT: integer range 0 to SDRAM_MAX := 0;
    constant SDRAM_IDLE: integer range 0 to SDRAM_MAX := 1;
    constant SDRAM_ACTIVATING: integer range 0 to SDRAM_MAX := 2;
    constant SDRAM_BANK_ACTIVE: integer range 0 to SDRAM_MAX := 3;
    constant SDRAM_WRITING: integer range 0 to SDRAM_MAX := 4;
    constant SDRAM_WRITE_WAIT: integer range 0 to SDRAM_MAX := 5;
    constant SDRAM_READING: integer range 0 to SDRAM_MAX := 6;
    constant SDRAM_READ_WAIT: integer range 0 to SDRAM_MAX := 7;
    constant SDRAM_REFRESH: integer range 0 to SDRAM_MAX := 8;
    constant SDRAM_REFRESH_WAIT: integer range 0 to SDRAM_MAX := 9;
    constant SDRAM_PRECHARGE: integer range 0 to SDRAM_MAX := 10;
    constant SDRAM_PRECHARGE_WAIT: integer range 0 to SDRAM_MAX := 11;
    signal sdram_mode: integer range 0 to SDRAM_MAX := SDRAM_UNINIT;

    signal sdram_active_bank: std_logic_vector(1 downto 0);
    signal sdram_active_row: std_logic_vector(10 downto 0);
    signal sdram_active_column: std_logic_vector(7 downto 0);

	component gowin_sdram
		port (
			O_sdram_clk: out std_logic;
			O_sdram_cke: out std_logic;
			O_sdram_cs_n: out std_logic;
			O_sdram_cas_n: out std_logic;
			O_sdram_ras_n: out std_logic;
			O_sdram_wen_n: out std_logic;
			O_sdram_dqm: out std_logic_vector(3 downto 0);
			O_sdram_addr: out std_logic_vector(10 downto 0);
			O_sdram_ba: out std_logic_vector(1 downto 0);
			IO_sdram_dq: inout std_logic_vector(31 downto 0);
			I_sdrc_rst_n: in std_logic;
			I_sdrc_clk: in std_logic;
			I_sdram_clk: in std_logic;
			I_sdrc_cmd_en: in std_logic;
			I_sdrc_cmd: in std_logic_vector(2 downto 0);
			I_sdrc_precharge_ctrl: in std_logic;
			I_sdram_power_down: in std_logic;
			I_sdram_selfrefresh: in std_logic;
			I_sdrc_addr: in std_logic_vector(20 downto 0);
			I_sdrc_dqm: in std_logic_vector(3 downto 0);
			I_sdrc_data: in std_logic_vector(31 downto 0);
			I_sdrc_data_len: in std_logic_vector(7 downto 0);
			O_sdrc_data: out std_logic_vector(31 downto 0);
			O_sdrc_init_done: out std_logic;
			O_sdrc_cmd_ack: out std_logic
		);
	end component;
begin
    I_sdrc_rst_n <= not reset;
    I_sdrc_clk <= clock;
    I_sdram_clk <= clock;
    I_sdram_selfrefresh <= '0';
    I_sdram_power_down <= '0';
    I_sdrc_precharge_ctrl <= '0'; --don't do precharge command always after reads and writes
    I_sdrc_data_len <= x"00"; --one byte

    process (all)
    begin
        if rambits=3 then
            I_sdrc_data <= wb_d_mosi & wb_d_mosi & wb_d_mosi & wb_d_mosi;
            case wb_addr(1 downto 0) is
                when "00" =>
                    wb_d_miso <= O_sdrc_data(7 downto 0);
                    I_sdrc_dqm <= "0001";
                when "01" => 
                    wb_d_miso <= O_sdrc_data(15 downto 8);
                    I_sdrc_dqm <= "0010";
                when "10" => 
                    wb_d_miso <= O_sdrc_data(23 downto 16);
                    I_sdrc_dqm <= "0100";
                when others => 
                    wb_d_miso <= O_sdrc_data(31 downto 24);
                    I_sdrc_dqm <= "1000";
            end case;
        else
            if not wb_addr(0) then
                I_sdrc_dqm <= "0011";
            else
                I_sdrc_dqm <= "1100";
            end if;
            I_sdrc_data <= wb_d_mosi & wb_d_mosi;
        end if;
    end process;

    process (clock)
    begin
        if rising_edge(clock) then
            if refresh_counter = 1 then
                refresh_needed <= '1';
            end if;
            if refresh_counter /= 0 then
                refresh_counter <= refresh_counter - 1;
            end if;
            case mode is
                when MODE_UNINIT =>
                    if O_sdrc_init_done then
                        mode <= MODE_IDLE;
                    end if;
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
            if sdram_mode = SDRAM_UNINIT then
                if O_sdrc_init_done = '1' then
                    sdram_mode <= SDRAM_IDLE;
                end if;
            elsif refresh_needed then
                case sdram_mode is
                    when SDRAM_IDLE => sdram_mode <= SDRAM_REFRESH;
                    when SDRAM_BANK_ACTIVE => sdram_mode <= SDRAM_PRECHARGE;
                    when SDRAM_REFRESH => sdram_mode <= SDRAM_REFRESH_WAIT;
                    when SDRAM_REFRESH_WAIT =>
                        if O_sdrc_cmd_ack then
                            sdram_mode <= SDRAM_IDLE;
                            refresh_row <= std_logic_vector(unsigned(refresh_row) + 1);
                            refresh_needed <= '0';
                        end if;
                    when others => null;
                end case;
            else
                case sdram_mode is
                    when SDRAM_IDLE =>
                        if wb_cyc then
                            sdram_mode <= SDRAM_ACTIVATING;
                            if rambits=3 then
                                I_sdrc_addr <= "00000000" & wb_addr(22 downto 10);
                            elsif rambits=4 then
                                I_sdrc_addr <= "00000000" & wb_addr(21 downto 9);
                            end if;
                        end if;
                    when others => null;
                end case;
            end if;
        end if;
    end process;

    process (all)
    begin
        case sdram_mode is
            when SDRAM_UNINIT | SDRAM_IDLE => I_sdrc_cmd <= "111"; --nop
            when SDRAM_REFRESH => I_sdrc_cmd <= "001";
            when SDRAM_REFRESH_WAIT => I_sdrc_cmd <= "111";
            when SDRAM_ACTIVATING => I_sdrc_cmd <= "011";
            when others => I_sdrc_cmd <= "111";
        end case;
    end process;

    --TODO
    --I_sdrc_addr
    --I_sdrc_cmd_en
    --I_sdrc_precharge_ctrl
    --O_sdrc_cmd_ack

	sdram: gowin_sdram port map (
		O_sdram_clk => O_sdram_clk,
		O_sdram_cke => O_sdram_cke,
		O_sdram_cs_n => O_sdram_cs_n,
		O_sdram_cas_n => O_sdram_cas_n,
		O_sdram_ras_n => O_sdram_ras_n,
		O_sdram_wen_n => O_sdram_wen_n,
		O_sdram_dqm => O_sdram_dqm,
		O_sdram_addr => O_sdram_addr,
		O_sdram_ba => O_sdram_ba,
		IO_sdram_dq => IO_sdram_dq,
		I_sdrc_rst_n => I_sdrc_rst_n,
		I_sdrc_clk => I_sdrc_clk,
		I_sdram_clk => I_sdram_clk,
		I_sdrc_cmd_en => I_sdrc_cmd_en,
		I_sdrc_cmd => I_sdrc_cmd,
		I_sdrc_precharge_ctrl => I_sdrc_precharge_ctrl,
		I_sdram_power_down => I_sdram_power_down,
		I_sdram_selfrefresh => I_sdram_selfrefresh,
		I_sdrc_addr => I_sdrc_addr,
		I_sdrc_dqm => I_sdrc_dqm,
		I_sdrc_data => I_sdrc_data,
		I_sdrc_data_len => I_sdrc_data_len,
		O_sdrc_data => O_sdrc_data,
		O_sdrc_init_done => O_sdrc_init_done,
		O_sdrc_cmd_ack => O_sdrc_cmd_ack
	);
end Behavioral;

