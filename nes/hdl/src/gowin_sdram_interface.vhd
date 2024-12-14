library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity gowin_sdram_interface is
	Generic(
        rambits: integer := 8);
    Port (
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
		wb_d_miso: out std_logic_vector(rambits-1 downto 0);
		wb_d_mosi: in std_logic_vector(rambits-1 downto 0);
		wb_err: out std_logic;
		wb_addr: in std_logic_vector((rambits/8)+20 downto 0);
		wb_bte: in std_logic_vector(1 downto 0);
		wb_cti: in std_logic_vector(2 downto 0);
		wb_cyc: in std_logic;
		wb_sel: in std_logic_vector((rambits/8)-1 downto 0);
		wb_stb: in std_logic;
		wb_we: in std_logic);
end gowin_sdram_interface;

architecture Behavioral of gowin_sdram_interface is

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

