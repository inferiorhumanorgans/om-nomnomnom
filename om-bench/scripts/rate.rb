#!/usr/bin/env ruby

require 'json'
# https://unicode-table.com/en/blocks/box-drawing/
if ARGV[0].nil?
  exit 1
end

lines = []
ARGV.each do |file|
  %w(python openmetrics-parser om-nomnomnom).each do |lib|
    filename = "target/criterion/should_pass/#{file}/#{lib}/base/estimates.json"
    stats = JSON.parse(File.read(filename))
    lines.push "%-35s %7d / sec" % ["#{file}/#{lib}", ( 1 / ( stats['slope']['point_estimate'] * 1e-9 ) ).round]
  end
  lines.push ""
end

width = lines.map {|l| l.length}.max
STDOUT.write "\u{2554}"
STDOUT.write "\u{2550}"* (width+2)
STDOUT.write "\u{2557}"
STDOUT.puts

lines.each_with_index do |line, idx|
  if line.empty?
    if idx < lines.length - 1
      puts "\u{2560}%-#{width+2}s\u{2562}" % ("\u{2500}"* (width+2))
    end
  else
    puts "\u{2551} %-#{width}s \u{2551}" % line
  end
end

STDOUT.write "\u{255A}"
STDOUT.write "\u{2550}"* (width+2)
STDOUT.write "\u{255D}"
STDOUT.puts
