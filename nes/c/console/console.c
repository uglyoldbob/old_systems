/*
 * console.c
 *
 * Created: 3/6/2017 10:29:02 AM
 *  Author: tepperson
 */ 

#include "console.h"
#include "../fatfs/src/ff.h"
#include "../nes/cartridge.h"
#include "../uart.h"

typedef void (*str_prc)(char *dat);

static int console_mode;

#define CONSOLE_MODE_MAIN 0
#define CONSOLE_MODE_NES_TESTS 1

static uint16_t num_tests = 0;
static uint16_t test_to_run = 0;

static FILINFO fno;
char *get_name(char* path, uint16_t index)
{
	FRESULT res;
	DIR dir;
	
	uint16_t num_roms = 0;
	res = f_opendir(&dir, path);                       /* Open the directory */
	if (res == FR_OK) {
		for (;;) {
			res = f_readdir(&dir, &fno);                   /* Read a directory item */
			if (res != FR_OK || fno.fname[0] == 0) break;  /* Break on error or end of dir */
			if (fno.fattrib & AM_DIR)
			{	/* It is a directory */
			}
			else
			{   /* It is a file. */
				if (nes_rom_test(fno.fname))
				{
					num_roms++;
					if (num_roms == index)
						return fno.fname;
				}
			}
		}
		f_closedir(&dir);
	}
	return "ERROR";
}

uint16_t scan_files (char* path, str_prc process)
{
	FRESULT res;
	DIR dir;
	UINT i;
	static FILINFO fno;
	uint16_t num_roms = 0;
	res = f_opendir(&dir, path);                       /* Open the directory */
	if (res == FR_OK) {
		for (;;) {
			res = f_readdir(&dir, &fno);                   /* Read a directory item */
			if (res != FR_OK || fno.fname[0] == 0) break;  /* Break on error or end of dir */
			if (fno.fattrib & AM_DIR) 
			{	/* It is a directory */
				path[i] = 0;
			}
			else
			{   /* It is a file. */
				if (nes_rom_test(fno.fname))
				{
					num_roms++;
					if (process != 0)
						process(fno.fname);
				}
			}
		}
		f_closedir(&dir);
	}

	return num_roms;
}

void print_rom_name(char *name)
{
	uart_send("(");
	uart_print(++num_tests);
	uart_send(") ");
	uart_send(name);
	uart_send("\r\n");
}

void count_tests()
{
	f_chdir("/test");
	uart_send("Which test would you like to run?\r\n");
	num_tests = 0;
	num_tests = scan_files(".", print_rom_name);
	f_chdir("..");
}

void handle_input(char dat)
{
	char temp[2];
	switch (console_mode)
	{
		case CONSOLE_MODE_MAIN:
			switch(dat)
			{
				case 't': case 'T':
					test_to_run = 0;
					count_tests();
					console_mode = CONSOLE_MODE_NES_TESTS;
					break;
				case 's': case 'S':
					p6502_step();
					break;
				case 'r': case'R':
					p6502_resume();
					break;
				case 'b': case 'B':
					p6502_break();
					break;
			}
			break;
		case CONSOLE_MODE_NES_TESTS:
			switch(dat)
			{
				case '0': case '1': case '2': case '3': case '4': 
				case '5': case '6': case '7': case '8': case '9':
					temp[0] = dat;
					temp[1] = 0;
					uart_send(temp);
					test_to_run *= 10;
					test_to_run += (dat) - '0';
					break;
				case 0x0D:
					uart_send("\r\nRunning test ");
					uart_print(test_to_run);
					uart_send (" ");
					f_chdir("/test");
					uart_send(get_name(".", test_to_run));
					nes_reset_power_off();
					nes_remove_cartridge();
					load_cartridge_image(get_name(".", test_to_run));
					psram_to_lcd();
					nes_power_on();
					f_chdir("..");
					uart_send("\r\n");
					console_mode = CONSOLE_MODE_MAIN;
					break;
			}
			break;
		default:
			console_mode = 0;
			break;
	}
	
}

void setup_console()
{
	console_mode = CONSOLE_MODE_MAIN;
	set_uart_receive(handle_input);	
}