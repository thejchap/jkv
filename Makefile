dev:
	@pkill -f nginx || true
	@PORT=3001 ./volume.sh /tmp/volume1/ &
	@PORT=3002 ./volume.sh /tmp/volume2/ &
	@PORT=3003 ./volume.sh /tmp/volume3/ &
	@PORT=3004 ./volume.sh /tmp/volume4/ &
	@PORT=3005 ./volume.sh /tmp/volume5/ &
	@cargo r -- --volumes \
	http://localhost:3001,\
	http://localhost:3002,\
	http://localhost:3003,\
	http://localhost:3004,\
	http://localhost:3005
