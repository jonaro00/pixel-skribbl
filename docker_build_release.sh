cargo b -r
cd frontend
trunk build --release
cd ..

docker build -t pixel-skribbl .
docker tag pixel-skribbl pixelskribbl.azurecr.io/pixel-skribbl
# docker login
# docker push
