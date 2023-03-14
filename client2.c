
#include <arpa/inet.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <sys/socket.h>
#include <unistd.h>
#include <sys/ioctl.h>

#define PORT 4000
  
int main(int argc, char const* argv[])
{
    int status, valread, client_fd;
    struct sockaddr_in serv_addr;
    char* hello = "HELO 2019A7PS0006G";
    char* fileget = "FILE ";


    FILE *fptr;

    char buffer[1024] = { 0 };
    if ((client_fd = socket(AF_INET, SOCK_STREAM, 0)) < 0) {
        printf("\n Socket creation error \n");
        return -1;
    }
  
    serv_addr.sin_family = AF_INET;
    serv_addr.sin_port = htons(PORT);
  
    // Convert IPv4 and IPv6 addresses from text to binary
    // form
    if (inet_pton(AF_INET, "127.0.0.1", &serv_addr.sin_addr)
        <= 0) {
        printf(
            "\nInvalid address/ Address not supported \n");
        return -1;
    }
  
    if ((status
         = connect(client_fd, (struct sockaddr*)&serv_addr,
                   sizeof(serv_addr)))
        < 0) {
        printf("\nConnection Failed \n");
        return -1;
    }
    send(client_fd, hello, strlen(hello), 0);
    printf("Hello message sent\n");
    valread = read(client_fd, buffer, 1024);
    printf("%s\n", buffer);
    send(client_fd, fileget, strlen(fileget), 0);


    char *p;
    p = strtok(buffer, " ");


    p = strtok(NULL, " ");

    printf("%s",p);
    long totalFileSize = strtol(p,NULL,10);
    printf("--->%ld",totalFileSize);

    char data[totalFileSize];
    memset( data, 0, totalFileSize*sizeof(char) );


    buffer[5] = '\0';

    read(client_fd,buffer,5);

    printf("this is in the buffer now:%s\n%s\n",buffer,data);

        long count=0;

        while(totalFileSize>0)
        {
            char buffer2[2048] = { 0 };
            valread = read(client_fd, buffer2, 2048);   

            for(int cc=0;cc<valread;cc++)
                data[cc+count]=buffer2[cc];
            count=count+valread;
            //strcat(data,buffer2);
            //strcat(data,"");
            if(totalFileSize<5000)
            {
                for(int ii=0;ii<valread;ii++)
                printf("%02hhx",buffer2[ii]);
            }
            else
            {
                            printf("Data:%ld\tBuffer2:%ld\tTotal file size:%ld\n",sizeof(data),sizeof(buffer2),totalFileSize);

            }
            totalFileSize=totalFileSize-valread;
        }



    printf("We are about to write to file now\n");
    //printf("Last 100 guys in data are%02hhx%02hhx",data[sizeof(data)-1],data[sizeof(data)-2]);
    for(int ii=1000;ii>=0;ii--)
    {
        printf("%02hhx",data[sizeof(data)-ii]);
        if(ii%2!=0)
            printf(" ");
        if((ii+1)%16==0 && ii>10)
            printf("\n");
    }
     fptr = fopen("./fileopen.png","wb");
     fwrite(&data, sizeof(data),1,fptr);
     fclose(fptr);

     int checksum=0;
     int sum=0;
     for(int ii=0;ii<sizeof(data);ii++)
     {
        checksum=checksum+data[ii];
     }
     printf("Checksum:%d",checksum);
     checksum=checksum&0xff;
    printf("Checksum:%hhx",checksum);
    checksum=~checksum+1;
    printf("Checksum:%hhx",checksum);
    if(checksum<0)
        checksum=checksum+256;
    printf("Checksum:%d",checksum);



        char* abra = "ABRA ";
        char checks[4];
        sprintf(checks,"%d",checksum);
        char toSend[100];
        strcpy(toSend,abra);
        strcat(toSend,checks);
        printf("\nToSend:%s\n",toSend);
        send(client_fd, toSend, strlen(toSend), 0);

    valread = read(client_fd, buffer, 1024);
    printf("Finally:%s\n",buffer);        

}