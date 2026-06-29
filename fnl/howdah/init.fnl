(local howdah {})

(fn howdah.connect 
  []
  (set howdah.channel 
       (vim.fn.jobstart 
         ["/Users/noahmoss/Projects/howdah/target/debug/howdah-server"]
         {:rpc true})))

(fn howdah.ping 
  []
  (vim.rpcrequest howdah.channel "ping"))

(fn howdah.query [sql]
  (vim.rpcrequest howdah.channel "query" sql))

(comment
  (howdah.connect)
  (howdah.query "select * from bird"))

howdah
