set dotenv-load

alias r := run
alias d := deploy
alias db := docker-build

run: ts
    cargo run

ts:
    tsc --build ./tsconfig.json

docker-build:
    docker build -t verrdon/meet:deploy-latest .

ecr: docker-build
    aws ecr get-login-password --profile=$AWS_PROFILE --region $AWS_REGION | docker login --username AWS --password-stdin $ECR_DNS_NAME
    docker tag verrdon/meet:deploy-latest $ECR_DNS_NAME/verrdon/meet:deploy-latest
    docker push $ECR_DNS_NAME/verrdon/meet:deploy-latest

deploy: ecr
    ssh -i $SSH_CERT ec2-user@$SERVER_ADDRESS ./deploy.sh
    
coturn:
    docker run -d --network=host -e DETECT_EXTERNAL_IP=yes -e DETECT_RELAY_IP=yes coturn/coturn -n --log-file=stdout --min-port=49160 --max-port=49200 --lt-cred-mech --fingerprint --no-multicast-peers --no-cli --no-tlsv1 --no-tlsv1_1 --realm=meet.verrdon.com --user=$COTURN_CREDS
