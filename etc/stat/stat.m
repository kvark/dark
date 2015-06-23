close all; clear all; clc;

data = csvread('book1');
symbols = [0:255];


figure(1);
plot(dist_per_sym);
xlabel('Symbol');
ylabel('Avg Distance');
title('Avg distance per symbol');