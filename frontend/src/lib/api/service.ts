export const checkAvailability =  () => {
    console.log("Checking availability")
    return fetch(`/health`).then((res) => {
        return res.ok
    }).catch((e) => {
        console.log(e);
        return false;
    })
    
}