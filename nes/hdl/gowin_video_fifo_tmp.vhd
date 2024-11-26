--Copyright (C)2014-2024 Gowin Semiconductor Corporation.
--All rights reserved.
--File Title: Template file for instantiation
--Tool Version: V1.9.9.03 Education
--Part Number: GW2AR-LV18QN88C8/I7
--Device: GW2AR-18
--Device Version: C
--Created Time: Tue Nov 26 15:29:30 2024

--Change the instance name and port connections to the signal names
----------Copy here to design--------

component gowin_video_fifo
	port (
		Data: in std_logic_vector(23 downto 0);
		WrClk: in std_logic;
		RdClk: in std_logic;
		WrEn: in std_logic;
		RdEn: in std_logic;
		Q: out std_logic_vector(23 downto 0);
		Empty: out std_logic;
		Full: out std_logic
	);
end component;

your_instance_name: gowin_video_fifo
	port map (
		Data => Data,
		WrClk => WrClk,
		RdClk => RdClk,
		WrEn => WrEn,
		RdEn => RdEn,
		Q => Q,
		Empty => Empty,
		Full => Full
	);

----------Copy end-------------------
