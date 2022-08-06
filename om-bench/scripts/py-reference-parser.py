try:
  metrics = prometheus_client.text_string_to_metric_families(data)
  count = 0
  for family in metrics:
    for sample in family.samples:
      count += 1
except:
  exit
