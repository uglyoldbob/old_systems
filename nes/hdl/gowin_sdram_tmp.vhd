--Copyright (C)2014-2024 Gowin Semiconductor Corporation.
--All rights reserved.
--File Title: Template file for instantiation
--Tool Version: V1.9.9.03 Education
--Part Number: GW2AR-LV18QN88C8/I7
--Device: GW2AR-18
--Device Version: C
--Created Time: Mon Dec  2 08:33:24 2024

--Change the instance name and port connections to the signal names
----------Copy here to design--------

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

your_instance_name: gowin_sdram
	port map (
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

----------Copy end-------------------
