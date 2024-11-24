library IEEE;
use ieee.std_logic_1164.all;
use ieee.std_logic_misc.all;
use IEEE.NUMERIC_STD.ALL;

entity resize_kernel3 is
	port (
		mode: in std_logic_vector(0 downto 0);
		din_a: in std_logic_vector(23 downto 0);
		din_b: in std_logic_vector(23 downto 0);
		din_c: in std_logic_vector(23 downto 0);
		din_d: in std_logic_vector(23 downto 0);
		din_e: in std_logic_vector(23 downto 0);
		din_f: in std_logic_vector(23 downto 0);
		din_g: in std_logic_vector(23 downto 0);
		din_h: in std_logic_vector(23 downto 0);
		din_i: in std_logic_vector(23 downto 0);
		dout_a: out std_logic_vector(23 downto 0);
		dout_b: out std_logic_vector(23 downto 0);
		dout_c: out std_logic_vector(23 downto 0);
		dout_d: out std_logic_vector(23 downto 0);
		dout_e: out std_logic_vector(23 downto 0);
		dout_f: out std_logic_vector(23 downto 0);
		dout_g: out std_logic_vector(23 downto 0);
		dout_h: out std_logic_vector(23 downto 0);
		dout_i: out std_logic_vector(23 downto 0);
		trigger: in std_logic;
		clock: in std_logic);
end resize_kernel3;

architecture Behavioral of resize_kernel3 is
begin
	process (clock)
	begin
		if rising_edge(clock) then
			if trigger then
				case mode is
					when "0" =>
						dout_a <= din_e;
						dout_b <= din_e;
						dout_c <= din_e;
						dout_d <= din_e;
						dout_e <= din_e;
						dout_f <= din_e;
						dout_g <= din_e;
						dout_h <= din_e;
						dout_i <= din_e;
					when "1" =>
						if din_d = din_b and din_d /= din_h and din_b /= din_f then
							dout_a <= din_d;
						else
							dout_a <= din_e;
						end if;
						if (din_d = din_b and din_d /= din_h and din_b /= din_f and din_e /= din_c) or
							(din_b = din_f and din_b /= din_d and din_f /= din_h and din_e /= din_a) then
							dout_b <= din_b;
						else
							dout_b <= din_e;
						end if;
						if din_b = din_f and din_b /= din_d and din_f /= din_h then
							dout_c <= din_f;
						else
							dout_c <= din_e;
						end if;
						if (din_h = din_d and din_h /= din_f and din_d /= din_b and din_e /= din_a) or
							(din_d = din_b and din_d /= din_h and din_b /= din_f and din_e /= din_g) then
							dout_d <= din_d;
						else
							dout_d <= din_e;
						end if;
						dout_e <= din_e;
						if (din_b = din_f and din_b /= din_d and din_f /= din_h and din_e /= din_i) or
							(din_f = din_h and din_f /= din_b and din_h /= din_d and din_e /= din_d) then
							dout_f <= din_f;
						else
							dout_f <= din_e;
						end if;
						if din_h = din_d and din_h /= din_f and din_d /= din_b then
							dout_g <= din_d;
						else
							dout_g <= din_e;
						end if;
						if (din_f = din_h and din_f /= din_b and din_h /= din_d and din_e /= din_g) or
							(din_h = din_d and din_h /= din_f and din_d /= din_b and din_e /= din_i) then
							dout_h <= din_h;
						else
							dout_h <= din_e;
						end if;
						if din_f = din_h and din_f /= din_b and din_h /= din_d then
							dout_i <= din_f;
						else
							dout_i <= din_e;
						end if;
					when others =>
				end case;
			end if;
		end if;
	end process;
end Behavioral;