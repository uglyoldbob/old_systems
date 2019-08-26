/*
 * processor_6502.h
 *
 * Created: 2/7/2017 1:21:56 PM
 *  Author: tepperson
 */ 


#ifndef PROCESSOR_6502_H_
#define PROCESSOR_6502_H_

#include <stdint.h>

void p6502_power_on();
uint8_t p6502_run_cycle();
void p6502_resume();

void p6502_step();
void p6502_break();

uint8_t p6502_single_step;

typedef uint8_t (*mem_read_callback)(uint16_t addr);
typedef void (*mem_write_callback)(uint16_t addr, uint8_t val);

void set_mapper_cpu(mem_read_callback r, mem_write_callback w);
void set_mapper_ppu(mem_read_callback r, mem_write_callback w);

#endif /* PROCESSOR_6502_H_ */