/*
 * nes.h
 *
 * Created: 2/10/2017 11:09:24 AM
 *  Author: tepperson
 */ 


#ifndef NES_H_
#define NES_H_

#include "processor_6502.h"

void nes_insert_cartridge(unsigned char *img, uint32_t length);
void nes_power_on(void);
void nes_reset(void);
void nes_reset_power_off(void);
void nes_run_tests(void);

#endif /* NES_H_ */